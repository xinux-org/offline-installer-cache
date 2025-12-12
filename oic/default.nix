{
  crane,
  xeonitte,
  pkgs,
}: let
  craneLib = crane.mkLib pkgs;
  src = craneLib.cleanCargoSource ./.;
  xeonitte-package = xeonitte.packages.${pkgs.stdenv.hostPlatform.system}.default;
  commonArgs = {
    inherit src;
    strictDeps = true;

    buildInputs = [];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  package = craneLib.buildPackage (
    commonArgs
    // {
      inherit cargoArtifacts;
    }
  );
in
  package
