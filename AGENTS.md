## Rules

- Tests must protect an observable contract, catch a realistic defect and use independent expected results. Exercise the relevant production path; do not duplicate the implementation or mock the behaviour being tested.
- Use `emulator -avd Light_Phone_III -writable-system` for the emulator
- Before reporting a visual defect or changing rendering code, verify the saved screenshot by comparing pixels in the affected region. An image preview alone is not sufficient evidence. If the preview and file evidence disagree, investigate the inspection tooling first.
