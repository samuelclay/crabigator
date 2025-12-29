fn main() {
    // Recompile when the hook script changes
    println!("cargo:rerun-if-changed=src/platforms/claude_code/stats_hook.py");
}
