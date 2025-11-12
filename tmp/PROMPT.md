Plan a refactor to migrate dhall to python based script to generate docker configuration.
In this project, we use dhall to generate docker configurations in the "./docker".
The dhall configuration can be found in the "./docker/.config".

What we want is to increase adoption and make it easy for new developer to onboard and contribute to the project.
Dhall is great, but many developers don't know about this tool.
We only use to generate docker config, so in theory, python should also be able to do this.

Dhall is typesafe, so we also need this new python script to be typesafe.
We need to make it easy for devs to manage and maintain these scripts.
We need to

- add python in the "development" devshell
- add pyright lsp in the devshell
- use pydantic for type validation
- integrate with just recipe for easy invocation

In the devshell, you should prefer to use native python nix dependencies directly like `python313Packages.pydantic`.
Do not create `pyproject.toml` file.

