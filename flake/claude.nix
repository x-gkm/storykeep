{
  perSystem = { pkgs, ... }: {
    devshells.default.packages = [ pkgs.claude-code ];
  };
}
