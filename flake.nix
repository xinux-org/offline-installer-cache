{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    xeonitte = {
      url = "github:xinux-org/xeonitte";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    xeonitte,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};

      xeonitte-package = xeonitte.packages.${pkgs.stdenv.hostPlatform.system}.default;
    in {
      devShells.default = pkgs.mkShell {
        name = "shell";

        buildInputs = with pkgs; [
          self.formatter.${system}

          deadnix
          nixd
          statix

          cargo
          rustc
          rust-analyzer
          clippy
          rustfmt
        ];

        shellHook = ''
          echo ${xeonitte-package}
        '';
      };

      formatter = pkgs.alejandra;

      packages = {
        default = self.packages.${system}.oic;
        oic = pkgs.callPackage ./oic { inherit crane xeonitte; };
      };
    });
}
