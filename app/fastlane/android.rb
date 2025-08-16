platform :android do
    desc "Build and release the app"
    lane :beta_android do |options|
      # Package name
      package_name = "im.phnx.prototype"
      track = "internal"
      gradle_propperties = {}
  
      # Determine if we should deploy to the Play Store
      upload_to_play_store = options[:upload_to_play_store]
  
      # We need to wrap the whole process in a begin/rescue block to ensure that we clean up the temporary files
      begin
        # We prepare the keystore and the Play Store key
        if upload_to_play_store
          # Decode the keystore from the base64 string and save it to a temporary file
          keystore_path = "release-key.jks"
          base64_keystore = ENV["ANDROID_KEYSTORE_BASE64"]
          UI.user_error!("ANDROID_KEYSTORE_BASE64 environment variable is missing!") if base64_keystore.nil?
          File.open(keystore_path, "wb") do |file|
            file.write(Base64.decode64(base64_keystore))
          end
  
          # Decode the Play Store key from the base64 string and save it to a temporary file
          playstore_key_path = "playstorekey.json"
          base64_playstore_key = ENV["ANDROID_PLAYSTORE_KEY_JSON"]
          UI.user_error!("ANDROID_PLAYSTORE_KEY_JSON environment variable is missing!") if base64_playstore_key.nil?
          File.open(playstore_key_path, "wb") do |file|
            file.write(Base64.decode64(base64_playstore_key))
          end
          
          # Get the previous build number
          previous_build_number = google_play_track_version_codes(
            track: track,
            package_name: package_name,
            json_key: "fastlane/" + playstore_key_path,
          )[0]
          current_build_number = previous_build_number + 1
  
          # Increment the build number in the gradle file
          increment_version_code(
            gradle_file_path: "android/app/build.gradle",
            version_code: current_build_number
          )

          # Prepare the signing properties
          gradle_propperties = {
            "android.injected.signing.store.file" => File.expand_path(keystore_path),
            "android.injected.signing.store.password" => ENV["ANDROID_KEY_PASSWORD"],
            "android.injected.signing.key.alias" => "upload",
            "android.injected.signing.key.password" => ENV["ANDROID_KEY_PASSWORD"]
          }
        end
     
        # We build the app with Flutter first to set up gradle
        sh "flutter precache --android"
        sh "flutter pub get"
        if upload_to_play_store
          sh "flutter build appbundle --release"
        else
          # Faster build for only one architecture
          sh "flutter build appbundle --target-platform android-arm64"
        end

        gradle(
          task: "bundle",
          build_type: "Release",
          project_dir: File.expand_path("../android"),
          properties: gradle_propperties
        )

        if upload_to_play_store
          # Upload to Google Play Store
          supply(
            validate_only: false,
            release_status: "draft",
            version_code: current_build_number,
            track: track,
            aab: "build/app/outputs/bundle/release/app-release.aab",
            json_key: "fastlane/" + playstore_key_path,
            package_name: package_name,
          )
        end
  
      rescue => e
        UI.error(e)
        raise
      ensure
        # Clean up the temporary files
         if upload_to_play_store
           File.delete(keystore_path)
           File.delete(playstore_key_path)
         end
      end
    end
  end
