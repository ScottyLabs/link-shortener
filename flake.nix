{
  description = "ScottyLabs link shortener";

  nixConfig = {
    extra-substituters = [ "https://scottylabs.cachix.org" ];
    extra-trusted-public-keys = [
      "scottylabs.cachix.org-1:hajjEX5SLi/Y7yYloiXTt2IOr3towcTGRhMh1vu6Tjg="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    scottylabs = {
      url = "git+https://codeberg.org/ScottyLabs/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs
    , scottylabs
    , ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          helpers = scottylabs.mkLib pkgs;

          web = helpers.buildDenoTask {
            src = ./sites/web;
            pname = "link-shortener-web";
            version = "0.1.0";
          };

          docs = helpers.buildMdbook {
            src = ./sites/docs;
            name = "link-shortener-docs";
          };

          link-shortener = helpers.buildRustService {
            src = ./.;
            pname = "link-shortener";
            version = "0.1.0";
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.makeWrapper
            ];
            buildInputs = [ pkgs.openssl ];
            # Bake the SPA into the binary so the service serves it
            buildArgs.postInstall = ''
              wrapProgram $out/bin/link-shortener --set STATIC_DIR ${web}
            '';
          };
        in
        {
          inherit link-shortener web docs;
        }
      );
    };
}
