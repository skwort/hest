{
  description = "hest - personal automation agent";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in {
        devShells.default = with pkgs;
          mkShell {
            buildInputs = with pkgs; [
              # Rust
              openssl
              pkg-config
              rust-bin.stable.latest.default

              # Python
              uv
              python313

              # Tools
              just
              prek
            ];

            shellHook = ''
              echo "hest dev env"
            '';
          };
      }
    );
}
