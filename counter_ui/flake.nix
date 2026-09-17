{
  description = "counter_ui — a view over counter_reader's LEZ chain reads";

  nixConfig = {
    extra-substituters = [ "https://cache.nix.logos.co/public" ];
    extra-trusted-public-keys = [ "public:l4HrXgL4nw246+LBh2SOJyhz64BoGegOYLheT/iIAPU=" ];
  };

  # Only the builder. counter_reader is declared in metadata.json so the host
  # loads it before showing this view, but a QML-only module runs no code
  # generators, so it needs none of the core's build outputs. To drive the real
  # core from `nix run .`, point the standalone at an installed modules dir:
  #   nix run . -- --modules-dir "$PWD/../run/modules" --load counter_reader
  inputs = {
    logos-module-builder.url = "github:logos-co/logos-module-builder/0.2.6";
  };

  outputs = inputs@{ logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosQmlModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
