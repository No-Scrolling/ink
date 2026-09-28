## Rules

- Use `emulator -avd Light_Phone_III -writable-system` for the emulator
- Before reporting a visual defect or changing rendering code, verify the saved screenshot with `scripts/agent-tools image`: compare pixels in the affected region and use OCR for text. An image preview alone is not sufficient evidence. If the preview and file evidence disagree, investigate the inspection tooling first. [Visual verification](docs/agent-tools.md#verify-a-visual-defect).

## Agent Tools

Use `scripts/agent-tools` for `experiment`, `bench`, `device`, `image` and `memory`. Output is compact by default; use `result ID --full` for evidence and `list --offset N` for older runs. Use `--background` then `wait ID --after CURSOR` for long runs. Reserve devices before changing them. [Usage](docs/agent-tools.md).
