{
  perSystem =
    { pkgs, ... }:
    {
      nixpkgs.config.allowUnfree = true;

      devshells.default.packages = [ pkgs.claude-code ];
    };
}
