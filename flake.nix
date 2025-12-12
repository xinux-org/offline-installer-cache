{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      devShells.default = pkgs.mkShell {
        name = "shell";

        buildInputs = with pkgs; [
          self.formatter.${system}

          cargo
          rustc
          rust-analyzer
          clippy
          rustfmt
        ];
      };

      formatter = pkgs.alejandra;

      packages = {
        default = self.packages.${system}.oic;
        oic = pkgs.callPackage ./oic { inherit crane; };
      };
    });
}
