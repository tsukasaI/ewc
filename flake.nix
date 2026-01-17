{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = { self, nixpkgs, git-hooks, ... }:
    let
      system = "aarch64-darwin";
      pkgs = nixpkgs.legacyPackages.${system};
      rustToolchain = pkgs.rustPlatform;
    in {
      checks.${system}.pre-commit = git-hooks.lib.${system}.run {
        src = ./.;
        hooks = {
          rustfmt = {
            enable = true;
            package = pkgs.rustfmt;
          };
          clippy = {
            enable = true;
            package = pkgs.clippy;
            settings.denyWarnings = true;
          };
          cargo-check = {
            enable = true;
            package = pkgs.cargo;
          };
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        inherit (self.checks.${system}.pre-commit) shellHook;
        buildInputs = [
          pkgs.cargo
          pkgs.rustc
          pkgs.rustfmt
          pkgs.clippy
        ] ++ self.checks.${system}.pre-commit.enabledPackages;
      };
    };
}
