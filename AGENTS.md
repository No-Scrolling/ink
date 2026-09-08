## Rules

- Use `emulator -avd Light_Phone_III -writable-system` for the emulator

## Agent Tools

Use `scripts/agent-tools` for `experiment`, `bench`, `device`, `image` and `memory`. Output is compact by default; use `result ID --full` for evidence and `list --offset N` for older runs. Use `--background` then `wait ID --after CURSOR` for long runs. Reserve devices before changing them. [Usage](docs/agent-tools.md).
