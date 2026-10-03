{ inputs, ... }:
{
  perSystem =
    { pkgs, lib, ... }:
    let
      rust-toolchain = pkgs.rust-bin.selectLatestNightlyWith (
        toolchain:
        toolchain.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        }
      );
    in
    {
      nixpkgs.overlays = [ inputs.rust-overlay.overlays.default ];

      devshells.default = {
        nativeBuildInputs = [ rust-toolchain ];
      };
    };

  flake-file.inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };
}
