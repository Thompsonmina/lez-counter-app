{
  description = "counter_reader — reads the lez_counter program's state from a LEZ sequencer";

  inputs = {
    # Release set v0.2.1-r.2+84ff81e (see playbook §2).
    logos-module-builder.url = "github:logos-co/logos-module-builder/0.2.6";
  };

  outputs = inputs@{ self, logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
