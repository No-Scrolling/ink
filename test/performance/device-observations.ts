export type Shell = (script: string, name: string) => Promise<string>;
export type ProcessSample = {
  phase: string; hostMonotonicMs: number; collectionDurationMs: number; pid: number;
  uptimeSeconds: number; clockTicksPerSecond: number; processStartTicks: number;
  userTicks: number; systemTicks: number; rssPages: number; totalCpuTicks: number;
  pssKiB: number; privateDirtyKiB: number | null; swapPssKiB: number | null;
  raw: { stat: string; cpu: string; status: string; meminfo: string };
  energy: EnergySample;
};
export type EnergySample = {
  status: "unavailable" | "available"; reason?: string;
  scope: "whole-device"; counters: Record<string, { value: number; unit: string; path: string }>;
  measurementGate: { physical: boolean; readableChargeOrEnergy: boolean; externalPower: boolean | null; discharging: boolean; eligible: boolean };
  raw: string;
};

const energyScript = `for base in /sys/class/power_supply/*; do
  [ -r "$base/type" ] || continue
  [ "$(cat "$base/type")" = Battery ] || continue
  for key in charge_counter energy_now current_now voltage_now status capacity; do
    [ -r "$base/$key" ] && printf '%s=%s\\n' "$base/$key" "$(cat "$base/$key")"
  done
done
dumpsys battery`;

export async function collectEnergy(shell: Shell, emulator: boolean, name: string): Promise<EnergySample> {
  if (emulator) return { status: "unavailable", reason: "Android emulator: synthetic battery and power values are excluded", scope: "whole-device", counters: {},
    measurementGate: { physical: false, readableChargeOrEnergy: false, externalPower: null, discharging: false, eligible: false }, raw: "" };
  const raw = await shell(energyScript, name);
  const counters: EnergySample["counters"] = {};
  for (const line of raw.split("\n")) {
    const match = line.match(/^(\/sys\/class\/power_supply\/[^=]+\/(charge_counter|energy_now|current_now|voltage_now))=(-?\d+)$/);
    if (match) counters[match[1]!] = { value: Number(match[3]), path: match[1]!, unit: ({ charge_counter: "µAh", energy_now: "µWh", current_now: "µA", voltage_now: "µV" })[match[2]!]! };
  }
  const charge = raw.match(/^\s*Charge counter:\s*(-?\d+)/m);
  if (charge && !Object.keys(counters).some(path => path.endsWith("/charge_counter"))) {
    counters["dumpsys battery:Charge counter"] = { value: Number(charge[1]), unit: "µAh", path: "dumpsys battery:Charge counter" };
  }
  const readableChargeOrEnergy = Object.entries(counters).some(([path, counter]) => /charge_counter|energy_now|Charge counter/.test(path) && counter.value > 0);
  const power = [...raw.matchAll(/^\s*(?:AC|USB|Wireless|Dock) powered:\s*(true|false)/gm)].map(match => match[1] === "true");
  const externalPower = power.length ? power.some(Boolean) : null;
  const discharging = /\/status=Discharging(?:\n|$)/.test(raw) || /^\s*status:\s*3\s*$/m.test(raw);
  return { status: readableChargeOrEnergy ? "available" : "unavailable",
    reason: readableChargeOrEnergy ? undefined : "No positive readable physical charge/energy counter; current and voltage alone do not establish energy", scope: "whole-device", counters,
    measurementGate: { physical: true, readableChargeOrEnergy, externalPower, discharging, eligible: readableChargeOrEnergy && externalPower === false && discharging }, raw };
}

export async function collectProcess(shell: Shell, packageName: string, phase: string, ticks: number, emulator: boolean): Promise<ProcessSample> {
  const began = performance.now();
  const raw = await shell(`pid=$(pidof ${packageName}); case "$pid" in ''|*' '*) exit 20;; esac
printf 'PID=%s\\n' "$pid"
cat /proc/uptime
cat /proc/$pid/stat
head -1 /proc/stat
cat /proc/$pid/status
printf '\\nMEMINFO\\n'
dumpsys meminfo "$pid"`, `process-${phase}`);
  const [prefix, meminfo] = raw.split("\nMEMINFO\n");
  if (!prefix || !meminfo) throw Error(`Incomplete process observation: ${phase}`);
  const lines = prefix.split("\n"), pid = Number(lines[0]!.slice(4));
  const stat = lines[2]!, fields = stat.slice(stat.lastIndexOf(")") + 2).split(/\s+/);
  const cpu = lines[3]!, cpuFields = cpu.trim().split(/\s+/).slice(1).map(Number);
  const total = meminfo.match(/^\s*TOTAL\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)/m);
  const pss = meminfo.match(/TOTAL PSS:\s*(\d+)/)?.[1] ?? total?.[1];
  const swapPss = meminfo.match(/TOTAL SWAP PSS:\s*(\d+)/)?.[1] ?? total?.[4];
  if (!Number.isFinite(Number(pss)) || !/^\d+$/.test(String(pss))) throw Error(`No genuine PSS observation: ${phase}`);
  const energy = await collectEnergy(shell, emulator, `energy-${phase}`);
  const result = { phase, hostMonotonicMs: began, collectionDurationMs: performance.now() - began, pid,
    uptimeSeconds: Number(lines[1]!.split(" ")[0]), clockTicksPerSecond: ticks,
    processStartTicks: Number(fields[19]), userTicks: Number(fields[11]), systemTicks: Number(fields[12]), rssPages: Number(fields[21]),
    // Linux guest/guest_nice are already included in user/nice; do not count them twice.
    totalCpuTicks: cpuFields.slice(0, 8).reduce((sum, value) => sum + value, 0),
    pssKiB: Number(pss), privateDirtyKiB: total ? Number(total[2]) : null,
    swapPssKiB: swapPss === undefined ? null : Number(swapPss),
    raw: { stat, cpu, status: lines.slice(4).join("\n"), meminfo }, energy };
  if (![result.pid, result.uptimeSeconds, result.processStartTicks, result.userTicks, result.systemTicks, result.rssPages, result.totalCpuTicks].every(Number.isFinite)) throw Error(`Invalid /proc observation: ${phase}`);
  return result;
}

export function cpuIntervals(samples: ProcessSample[]) {
  return samples.slice(1).map((current, index) => {
    const previous = samples[index]!, durationSeconds = current.uptimeSeconds - previous.uptimeSeconds;
    if (current.pid !== previous.pid || current.processStartTicks !== previous.processStartTicks || durationSeconds <= 0) throw Error("Process restarted or the CPU observation interval is invalid");
    const processTicks = current.userTicks + current.systemTicks - previous.userTicks - previous.systemTicks;
    const systemTicks = current.totalCpuTicks - previous.totalCpuTicks;
    return { from: previous.phase, to: current.phase, durationSeconds, processTicks, totalCpuTicks: systemTicks,
      processCpuSeconds: processTicks / current.clockTicksPerSecond,
      oneCorePercent: processTicks / current.clockTicksPerSecond / durationSeconds * 100,
      totalMachinePercent: systemTicks > 0 ? processTicks / systemTicks * 100 : null };
  });
}

export function energyIntervals(samples: ProcessSample[]) {
  return samples.slice(1).map((current, index) => {
    const previous = samples[index]!, durationSeconds = current.uptimeSeconds - previous.uptimeSeconds;
    const counters = Object.entries(current.energy.counters).flatMap(([path, end]) => {
      const start = previous.energy.counters[path];
      return start && start.unit === end.unit ? [{ path, unit: end.unit, start: start.value, end: end.value, delta: end.value - start.value }] : [];
    });
    return { from: previous.phase, to: current.phase, durationSeconds, status: current.energy.status, scope: "whole-device", counters,
      dischargeGatePassed: previous.energy.measurementGate.eligible && current.energy.measurementGate.eligible,
      boundary: "Uncorrected physical device counters; USB charging, display, radios, OS and benchmark collection are included. No app-attributed joules or emulator energy estimate." };
  });
}
