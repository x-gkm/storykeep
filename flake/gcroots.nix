{
  perSystem =
    { pkgs, lib, ... }:
    let
      gcroots = pkgs.writeShellApplication {
        name = "gcroots";
        runtimeInputs = [
          pkgs.git
          pkgs.jq
        ];
        text = ''
          # Register GC roots so `nix-store --gc` keeps this flake's inputs and devshell.
          project=$(git rev-parse --show-toplevel)
          roots="$project/.nix/gcroots"
          mkdir -p "$project/.nix"
          next=$(mktemp -d "$project/.nix/gcroots.XXXXXX")

          i=0
          for path in $(nix flake archive --json "$project" | jq -r '.. | .path? // empty'); do
            nix-store --add-root "$next/input-$i" --realise "$path" >/dev/null
            i=$((i + 1))
          done
          echo "rooted $i flake inputs"

          nix develop "$project" --profile "$next/devshell" --command true
          echo "rooted devshell"

          rm -rf "$roots"
          mv "$next" "$roots"
        '';
      };
    in
    {
      apps.gcroots.program = lib.getExe gcroots;
    };
}
