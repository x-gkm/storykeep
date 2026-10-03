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
      options.nixpkgs = {
        overlays = lib.mkOption {
          type = lib.types.listOf overlayType;
          default = [ ];
        };
        config = lib.mkOption {
          type = lib.types.attrsOf lib.types.anything;
          default = { };
        };
      };

      config._module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        inherit (config.nixpkgs) overlays config;
      };
    }
  );

  config = {
    flake-file.inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };
}
