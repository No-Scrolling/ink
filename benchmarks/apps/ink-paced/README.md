# Continuous update benchmark

This fixture schedules 180 updates over three seconds, starting with 500 mounted text cells and spacing changes enabled. Each update changes every cell's label and every row's horizontal gap. It deliberately retains off-screen cells. It uses the same layout as `ink-updates`, with a timer driving the refresh action.

Build with `INK_BRIDGE_TIMING=1`, reserve a device and install through `scripts/agent-tools`. Run the `paced` probe against `com.vandam.benchmark.ink.paced` while the app is at rest. See [agent tools](../../../docs/agent-tools.md) for the commands and measurement limits.

Report commit counts and submission intervals alongside throughput. A rate near 60 per second does not establish under-16-ms update latency or guarantee that the panel displayed every update. Timers can fall behind and React can coalesce updates; the benchmark must not hide those outcomes.
