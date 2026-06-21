{
    description = "Nix flake for rust";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
        flake-parts.url = "github:hercules-ci/flake-parts";
    };
    
    outputs = inputs@{ flake-parts, ... }:
      flake-parts.lib.mkFlake { inherit inputs; } {
        systems = [
          "aarch64-darwin"
          "aarch64-linux"
          "x86_64-darwin"
          "x86_64-linux"
        ];

        perSystem = { pkgs, ... }: {
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustc
              cargo
              clippy
              rustfmt
              rust-analyzer
            ];
          };
        };
      };
}
