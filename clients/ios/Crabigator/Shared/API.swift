import Foundation
import Security

struct APIError: LocalizedError {
    var status: Int
    var message: String
    var errorDescription: String? { message }
}

/// App and notification extension use the same device-only Keychain access group.
final class Credentials {
    static let shared = Credentials()
    static let defaults = UserDefaults(suiteName: "group.com.newsblur.crabigator") ?? .standard
    private let lock = NSRecursiveLock()
    private var query: [String: Any] {
        var q: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: "com.newsblur.crabigator", kSecAttrAccount as String: "connection"]
        if let group = Bundle.main.object(forInfoDictionaryKey: "SharedKeychainGroup") as? String, !group.contains("$(") { q[kSecAttrAccessGroup as String] = group }
        return q
    }
    func connection() -> (origin: String, token: String) {
        lock.lock(); defer { lock.unlock() }
        var q = query; q[kSecReturnData as String] = true
        var result: CFTypeRef?
        guard SecItemCopyMatching(q as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data, let json = try? JSONSerialization.jsonObject(with: data) as? JSON else { return ("https://drinkcrabigator.com", "") }
        return (json.text("origin", "https://drinkcrabigator.com"), json.text("token"))
    }
    var mobileID: String {
        lock.lock(); defer { lock.unlock() }
        if let value = Self.defaults.string(forKey: "mobileID") { return value }
        let value = UUID().uuidString; Self.defaults.set(value, forKey: "mobileID"); return value
    }
    func save(origin: String, token: String) throws {
        lock.lock(); defer { lock.unlock() }
        let data = try JSONSerialization.data(withJSONObject: ["origin": origin, "token": token])
        let attributes: [String: Any] = [kSecValueData as String: data, kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly]
        let status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if status == errSecItemNotFound {
            let result = SecItemAdd(query.merging(attributes) { _, new in new } as CFDictionary, nil)
            guard result == errSecSuccess else { throw APIError(status: Int(result), message: "Could not save the pairing credential.") }; return
        }
        guard status == errSecSuccess else { throw APIError(status: Int(status), message: "Could not save the pairing credential.") }
    }
    func clear() { lock.lock(); defer { lock.unlock() }; SecItemDelete(query as CFDictionary) }
}

final class API: NSObject, URLSessionTaskDelegate {
    static let shared = API()
    let credentials = Credentials.shared
    lazy var session: URLSession = {
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 25
        config.timeoutIntervalForResource = 35
        config.urlCache = nil
        config.httpCookieStorage = nil
        return URLSession(configuration: config, delegate: self, delegateQueue: nil)
    }()
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) { completionHandler(nil) }
    static func origin(_ text: String) throws -> String {
        let value = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let u = URLComponents(string: value), u.scheme == "https", u.host != nil, u.user == nil, u.password == nil, u.query == nil, u.fragment == nil, ["", "/"].contains(u.path) else { throw APIError(status: 400, message: "Use an HTTPS server address.") }
        return value.hasSuffix("/") ? String(value.dropLast()) : value
    }
    static func pairingCode(_ text: String) -> String {
        let code = text.filter { $0.isLetter || $0.isNumber }.uppercased()
        guard code.count == 9 else { return text.trimmingCharacters(in: .whitespacesAndNewlines) }
        let a = Array(code); return stride(from: 0, to: 9, by: 3).map { String(a[$0..<$0 + 3]) }.joined(separator: "-")
    }
    func request(_ path: String, body: JSON? = nil) throws -> URLRequest {
        let connection = credentials.connection()
        guard let url = URL(string: connection.origin + path) else { throw APIError(status: 400, message: "Invalid server address.") }
        var request = URLRequest(url: url)
        request.setValue("Bearer " + connection.token, forHTTPHeaderField: "Authorization")
        if let body { request.httpMethod = "POST"; request.httpBody = try JSONSerialization.data(withJSONObject: body); request.setValue("application/json", forHTTPHeaderField: "Content-Type") }
        return request
    }
    func execute(_ request: URLRequest) async throws -> JSON {
        // Mutating requests are issued once. An ambiguous failure must never replay terminal input.
        let (data, response) = try await session.data(for: request)
        let json = (try? JSONSerialization.jsonObject(with: data)) as? JSON
        let code = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard (200..<300).contains(code) else { throw APIError(status: code, message: json?.text("error", "Request failed (\(code)).") ?? "Request failed (\(code)).") }
        guard let json else { throw APIError(status: code, message: "The server returned an unreadable response.") }
        return json
    }
    func call(_ path: String, body: JSON? = nil) async throws -> JSON { try await execute(request(path, body: body)) }
    func pair(_ code: String, origin: String, name: String) async throws {
        let origin = try Self.origin(origin)
        var request = URLRequest(url: URL(string: origin + "/api/pairing/claim")!)
        request.httpMethod = "POST"; request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: ["pairing_token": Self.pairingCode(code), "mobile_id": credentials.mobileID, "mobile_name": name])
        let result = try await execute(request), token = result.text("mobile_token")
        guard !token.isEmpty else { throw APIError(status: 400, message: "The server did not return a pairing credential.") }
        try credentials.save(origin: origin, token: token)
    }
    func snapshot(_ id: String) async throws -> JSON { try await call("/api/mobile/sessions/\(id)") }
    func action(_ id: String, _ action: TerminalAction, revision: Int? = nil) async throws {
        var body = action.body; body.removeValue(forKey: "expected_prompt_revision")
        if let revision { body["expected_prompt_revision"] = revision }
        _ = try await call("/api/sessions/\(id)/\(action.route)", body: body)
    }
    func transcribe(_ file: URL) async throws -> String {
        let boundary = UUID().uuidString
        var request = try request("/api/transcribe")
        request.httpMethod = "POST"; request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")
        var data = Data("--\(boundary)\r\nContent-Disposition: form-data; name=\"file\"; filename=\"recording.m4a\"\r\nContent-Type: audio/mp4\r\n\r\n".utf8)
        data.append(try Data(contentsOf: file)); data.append(Data("\r\n--\(boundary)--\r\n".utf8)); request.httpBody = data
        return try await execute(request).text("text").trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
