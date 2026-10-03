{
  perSystem =
    { pkgs, ... }:
    let
      androidSdk = (pkgs.androidenv.composeAndroidPackages { includeNDK = true; }).androidsdk;
      jdk = pkgs.jdk;
    in
    {
      nixpkgs.config = {
        allowUnfree = true;
        android_sdk.accept_license = true;
      };

      devshells.default = {
        packages = [
          pkgs.flutter
          androidSdk
          jdk
        ];

        env = rec {
          ANDROID_HOME = "${androidSdk}/libexec/android-sdk";
          ANDROID_SDK_ROOT = ANDROID_HOME;
          JAVA_HOME = jdk.home;
        };
      };
    };
}
