{ inputs, ... }:
{
  imports = [
    inputs.process-compose-flake.flakeModule
  ];

  perSystem =
    {
      config,
      pkgs,
      lib,
      self',
      ...
    }:
    let
      dbName = "storykeep";
    in
    {
      process-compose.backend-services = {
        imports = [
          inputs.services-flake.processComposeModules.default
        ];

        services.postgres.pg = {
          enable = true;
          port = 5433;
          superuser = "storykeep";
          initialDatabases = [
            {
              name = dbName;
            }
          ];
        };
      };

      devshells.default = {
        inputsFrom = [
          config.process-compose.backend-services.services.outputs.devShell
        ];

        packages = [
          self'.packages.backend-services
          pkgs.sqlx-cli
        ];

        env =
          let
            pg = config.process-compose.backend-services.services.postgres.pg;
          in
          rec {
            PGUSER = pg.superuser;
            PGHOST = pg.listen_addresses;
            PGPORT = toString pg.port;
            PGDATABASE = dbName;
            DATABASE_URL = "postgresql://${PGUSER}@${PGHOST}:${PGPORT}/${PGDATABASE}";
          };
      };
    };

  flake-file.inputs = {
    process-compose-flake.url = "github:Platonic-Systems/process-compose-flake";
    services-flake.url = "github:juspay/services-flake";
  };
}
