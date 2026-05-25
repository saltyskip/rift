Pod::Spec.new do |s|
  s.name             = 'rift_flutter_ffi'
  s.version          = '0.2.0'
  s.summary          = 'Rift deep link SDK for Flutter'
  s.description      = 'Attribution, deferred deep linking, and conversion tracking via Rift.'
  s.homepage         = 'https://riftl.ink'
  s.license          = { type: 'MIT', file: '../LICENSE' }
  s.author           = { 'Rift' => 'hi@riftl.ink' }
  s.source           = { path: '.' }

  s.source_files     = 'Classes/**/*.swift'
  s.swift_version    = '5.0'
  s.platform         = :ios, '12.0'
  s.dependency 'Flutter'

  # Prebuilt XCFramework containing the Rust staticlib + C header.
  # The release tarball places it alongside this podspec.
  s.vendored_frameworks = 'rift_flutter_ffi.xcframework'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
  }
end
