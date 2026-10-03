{ inputs, flake-parts-lib, ... }:
{
  options.perSystem = flake-parts-lib.mkPerSystemOption (
    {
      config,
      lib,
      system,
      ...
    }:
    let
      overlayType = lib.mkOptionType {
        name = "nixpkgs-overlay";
        description = "nixpkgs overlay";
        check = lib.isFunction;
        merge = lib.mergeOneOption;
      };
    in
    {
      options.nixpkgs.overlays = lib.mkOption {
        type = lib.types.listOf overlayType;
        default = [ ];
      };

      config._module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = config.nixpkgs.overlays;
        config = {
          allowUnfree = true;
          android_sdk.accept_license = true;
        };
      };
    }
  );

  config = {
    flake-file.inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };
}
