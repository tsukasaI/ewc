{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = { self, nixpkgs, git-hooks, ... }:
    let
      system = "aarch64-darwin";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      checks.${system}.pre-commit = git-hooks.lib.${system}.run {
        src = ./.;
        hooks = {
          rustfmt.enable = true;
          clippy = {
            enable = true;
            settings.denyWarnings = true;
          };
          cargo-check.enable = true;
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        inherit (self.checks.${system}.pre-commit) shellHook;
        buildInputs = self.checks.${system}.pre-commit.enabledPackages;
      };
    };
}
