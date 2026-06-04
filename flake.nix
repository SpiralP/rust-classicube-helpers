{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-utils.url = "github:SpiralP/nix-flake-utils";
  };

  outputs = inputs@{ flake-utils, ... }:
    flake-utils.lib.makeOutputs inputs
      ({ lib, pkgs, makeRustPackage, ... }:
        let
          src = lib.sourceByRegex ./. [
            "^Cargo\.(lock|toml)$"
            "^src(/.*)?$"
          ];

          nativeBuildInputs = with pkgs; [
            rustPlatform.bindgenHook
          ];
        in
        {
          default = makeRustPackage pkgs (self: {
            inherit src nativeBuildInputs;

            dontUseCargoParallelTests = true;
          });

          docs = makeRustPackage pkgs (self: {
            inherit src nativeBuildInputs;

            buildPhase = ''
              runHook preBuild
              cargo doc --no-deps
              runHook postBuild
            '';

            # tests run in the default build; the docs output only needs rustdoc
            doCheck = false;

            installPhase = ''
              runHook preInstall
              mv target/doc $out
              runHook postInstall
            '';
          });
        });
}
