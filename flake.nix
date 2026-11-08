{
  description = "ScottyLabs link shortener";

  nixConfig = {
    extra-substituters = [ "https://scottylabs.cachix.org" ];
    extra-trusted-public-keys = [
      "scottylabs.cachix.org-1:hajjEX5SLi/Y7yYloiXTt2IOr3towcTGRhMh1vu6Tjg="
    ];
  };

  inputs = {
    scottylabs.url = "git+https://git.cmu.dev/ScottyLabs/kennel";
    nixpkgs.follows = "scottylabs/nixpkgs";
    flake-parts.follows = "scottylabs/flake-parts";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.scottylabs.flakeModules.default ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        { pkgs, helpers, ... }:
        let
          web = helpers.buildDenoTask {
            src = ./sites/web;
            pname = "link-shortener-web";
            output = "build";
          };
        in
        {
          scottylabs = {
            project.name = "link-shortener";
            rust.enable = true;
            deno = {
              enable = true;
              svelte.enable = true;
              dirs = [ "sites/web" ];
            };
            postgres.enable = true;
            ricochet = {
              enable = true;
              appUrl = "http://localhost:5173";
            };

            shell = {
              packages = [
                pkgs.zensical
                pkgs.sea-orm-cli
              ];
              scripts = {
                migration = ''sea-orm-cli migrate generate "$1" -d crates/migration'';
                migrate = "sea-orm-cli migrate up -d crates/migration";
                generate-entities = "sea-orm-cli generate entity -o crates/entity/src --with-serde both --lib --model-extra-derives 'utoipa::ToSchema' --enum-extra-derives 'utoipa::ToSchema'";
                generate-api = "cd sites/web && deno task generate-api";
                docs = "cd sites/docs && zensical serve";
              };
            };
          };

          services.link-shortener = {
            package = helpers.buildRustService {
              src = ./.;
              pname = "link-shortener";
              version = "0.1.0";
            };
            customDomain = "cmu.lol";
            env.STATIC_DIR = "${web}";
          };

          sites.docs = {
            package = helpers.buildZensical {
              src = ./sites/docs;
              name = "link-shortener-docs";
            };
            customDomain = "docs.cmu.lol";
          };
        };
    };
}
