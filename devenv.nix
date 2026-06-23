{ pkgs, inputs, ... }:

{
  imports = [
    inputs.scottylabs.devenvModules.default
  ];

  scottylabs = {
    enable = true;
    project.name = "link-shortener";

    rust.enable = true;
    deno = {
      enable = true;
      svelte.enable = true;
    };
    postgres.enable = true;
    secrets.enable = true;
    ricochet = {
      enable = true;
      appUrl = "http://localhost:5173";
    };

    kennel.services.link-shortener.customDomain = "cmu.lol";
    kennel.sites.docs.customDomain = "docs.cmu.lol";
  };

  packages = with pkgs; [
    mdbook
    sea-orm-cli
  ];

  scripts = {
    migration.exec = ''sea-orm-cli migrate generate "$1" -d crates/migration'';
    migrate.exec = "sea-orm-cli migrate up -d crates/migration";
    generate-entities.exec = "sea-orm-cli generate entity -o crates/entity/src --with-serde both --lib --model-extra-derives 'utoipa::ToSchema' --enum-extra-derives 'utoipa::ToSchema'";
    generate-api.exec = "cd sites/web && deno task generate-api";
    docs.exec = "cd sites/docs && mdbook serve";
  };
}
