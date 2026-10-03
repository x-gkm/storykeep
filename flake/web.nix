{
  perSystem =
    { pkgs, ... }:
    {
      devshells.default.packages = [
        pkgs.bun
        pkgs.typescript-language-server
      ];
    };
}
