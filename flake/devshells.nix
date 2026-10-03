{ flake-parts-lib, ... }:
{
  options.perSystem = flake-parts-lib.mkPerSystemOption (
    {
      config,
      lib,
      pkgs,
      ...
    }:
    {
      options.devshells = lib.mkOption {
        type = lib.types.attrsOf (
          lib.types.submodule {
            options = {
              packages = lib.mkOption {
                type = lib.types.listOf lib.types.package;
                default = [ ];
              };
              nativeBuildInputs = lib.mkOption {
                type = lib.types.listOf lib.types.package;
                default = [ ];
              };
              inputsFrom = lib.mkOption {
                type = lib.types.listOf lib.types.package;
                default = [ ];
              };
              env = lib.mkOption {
                type = lib.types.attrsOf lib.types.str;
                default = { };
              };
            };
          }
        );
        default = { };
      };

      config.devShells = lib.mapAttrs (
        name: devshell:
        pkgs.mkShell {
          inherit name;
          inherit (devshell)
            packages
            nativeBuildInputs
            inputsFrom
            env
            ;
        }
      ) config.devshells;
    }
  );
}
