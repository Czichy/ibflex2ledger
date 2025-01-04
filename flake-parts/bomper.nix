_: {
  perSystem = {...}: {
    bomper = {
      enable = true;
      configuration = ''
        (
            cargo: Some(Autodetect),
            authors: Some({
                "Christian Czichy": "czichy"
            }),
        )
      '';
    };
  };
}
