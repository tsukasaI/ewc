{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = { self, nixpkgs, git-hooks, ... }:
    let
      supportedSystems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "ewc";
            version = "0.3.1";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            # Skip integration tests in Nix sandbox (requires filesystem access)
            # Unit tests (65 tests) still run
            cargoTestFlags = [ "--lib" ];

            meta = with pkgs.lib; {
              description = "Enhanced Word Count - A modern alternative to wc";
              homepage = "https://github.com/tsukasaI/ewc";
              license = licenses.mit;
              maintainers = [];
            };
          };
        }
      );

      checks = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in {
          pre-commit = git-hooks.lib.${system}.run {
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
        }
      );

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.mkShell {
            inherit (self.checks.${system}.pre-commit) shellHook;
            buildInputs = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.clippy
              pkgs.git-cliff
            ] ++ self.checks.${system}.pre-commit.enabledPackages;
          };
        }
      );
    };
}
