#!/usr/bin/env ruby
require 'xcodeproj'
require 'fileutils'
root = File.expand_path('..', __dir__)
project = Xcodeproj::Project.new(File.join(root, 'Crabigator.xcodeproj'))
app = project.new_target(:application, 'Crabigator', :ios, '17.0')
extension = project.new_target(:app_extension, 'NotificationService', :ios, '17.0')
tests = project.new_target(:unit_test_bundle, 'CrabigatorTests', :ios, '17.0')
ui_tests = project.new_target(:ui_test_bundle, 'CrabigatorUITests', :ios, '17.0')
shared = Dir[File.join(root, 'Crabigator/Shared/*.swift')]
[app, extension, tests, ui_tests].each do |target|
  target.build_configurations.each do |config|
    config.build_settings.merge!({
      'SWIFT_VERSION' => '5.0', 'TARGETED_DEVICE_FAMILY' => '1,2',
      'DEVELOPMENT_TEAM' => 'HR7P97SD72', 'CODE_SIGN_STYLE' => 'Automatic',
      'IPHONEOS_DEPLOYMENT_TARGET' => '17.0', 'CURRENT_PROJECT_VERSION' => '1',
      'MARKETING_VERSION' => '1.0', 'GENERATE_INFOPLIST_FILE' => 'NO',
      'ENABLE_USER_SCRIPT_SANDBOXING' => 'YES', 'SWIFT_EMIT_LOC_STRINGS' => 'YES',
      'ASSETCATALOG_COMPILER_APPICON_NAME' => target == app ? 'AppIcon' : '',
      'SWIFT_OPTIMIZATION_LEVEL' => config.name == 'Debug' ? '-Onone' : '-O',
      'SWIFT_ACTIVE_COMPILATION_CONDITIONS' => config.name == 'Debug' ? 'DEBUG' : '',
    })
  end
end
sources = Dir[File.join(root, 'Crabigator/**/*.swift')]
references = {}
(sources + Dir[File.join(root, 'NotificationService/*.swift')] + Dir[File.join(root, 'Tests/*.swift')] + Dir[File.join(root, 'UITests/*.swift')]).each do |file|
  path = file.delete_prefix(root + '/')
  references[file] = project.main_group.new_file(path)
end
app.add_file_references(sources.map { |f| references[f] })
extension.add_file_references((shared + Dir[File.join(root, 'NotificationService/*.swift')]).map { |f| references[f] })
tests.add_file_references(Dir[File.join(root, 'Tests/*.swift')].map { |f| references[f] })
ui_tests.add_file_references(Dir[File.join(root, 'UITests/*.swift')].map { |f| references[f] })
Dir[File.join(root, 'Crabigator/Resources/*')].each do |file|
  next if File.basename(file) == 'Info.plist'
  ref = project.main_group.new_file(file.delete_prefix(root + '/'))
  app.resources_build_phase.add_file_reference(ref)
end
app.build_configurations.each do |c|
  c.build_settings['PRODUCT_BUNDLE_IDENTIFIER'] = 'com.newsblur.crabigator'
  c.build_settings['INFOPLIST_FILE'] = 'Crabigator/Resources/Info.plist'
  c.build_settings['CODE_SIGN_ENTITLEMENTS'] = 'Crabigator/Crabigator.entitlements'
end
extension.build_configurations.each do |c|
  c.build_settings['PRODUCT_BUNDLE_IDENTIFIER'] = 'com.newsblur.crabigator.NotificationService'
  c.build_settings['INFOPLIST_FILE'] = 'NotificationService/Info.plist'
  c.build_settings['CODE_SIGN_ENTITLEMENTS'] = 'NotificationService/NotificationService.entitlements'
  c.build_settings['APPLICATION_EXTENSION_API_ONLY'] = 'YES'
  c.build_settings['SKIP_INSTALL'] = 'YES'
end
[tests, ui_tests].each do |t|
  t.add_dependency(app)
  t.build_configurations.each do |c|
    c.build_settings['PRODUCT_BUNDLE_IDENTIFIER'] = "com.newsblur.crabigator.#{t.name}"
    c.build_settings['GENERATE_INFOPLIST_FILE'] = 'YES'
    c.build_settings['TEST_TARGET_NAME'] = 'Crabigator'
    if t == tests
      c.build_settings['TEST_HOST'] = '$(BUILT_PRODUCTS_DIR)/Crabigator.app/$(BUNDLE_EXECUTABLE_FOLDER_PATH)/Crabigator'
      c.build_settings['BUNDLE_LOADER'] = '$(TEST_HOST)'
    end
  end
end
app.add_dependency(extension)
phase = app.new_copy_files_build_phase('Embed App Extensions')
phase.dst_subfolder_spec = '13'
phase.add_file_reference(extension.product_reference).settings = { 'ATTRIBUTES' => ['RemoveHeadersOnCopy'] }
project.root_object.attributes['TargetAttributes'] = { app.uuid => {'SystemCapabilities' => {'com.apple.Push' => {'enabled' => 1}, 'com.apple.ApplicationGroups.iOS' => {'enabled' => 1}}} }
project.save
scheme = Xcodeproj::XCScheme.new
scheme.add_build_target(app)
scheme.add_test_target(tests)
scheme.add_test_target(ui_tests)
scheme.set_launch_target(app)
scheme.save_as(project.path, 'Crabigator', true)
puts project.path
