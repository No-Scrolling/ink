use std::{fs, marker::PhantomData, mem, rc::Rc, sync::OnceLock};

const CPU_COUNT: usize = 1024;
type CpuSet = [usize; CPU_COUNT / usize::BITS as usize];

unsafe extern "C" {
    fn sched_getaffinity(pid: i32, size: usize, mask: *mut usize) -> i32;
    fn sched_setaffinity(pid: i32, size: usize, mask: *const usize) -> i32;
}

fn allowed() -> Option<CpuSet> {
    let mut mask = [0; CPU_COUNT / usize::BITS as usize];
    (unsafe { sched_getaffinity(0, mem::size_of_val(&mask), mask.as_mut_ptr()) } == 0)
        .then_some(mask)
}

fn apply(mask: &CpuSet) -> bool {
    unsafe { sched_setaffinity(0, mem::size_of_val(mask), mask.as_ptr()) == 0 }
}

/// Prefer the fastest permitted cores for this scope, restoring the thread mask on drop.
pub fn prefer_performance() -> Option<AffinityGuard> {
    static PREFERENCE: OnceLock<Option<CpuPreference>> = OnceLock::new();
    PREFERENCE.get_or_init(CpuPreference::discover).as_ref()?.enter()
}

struct CpuPreference {
    preferred: CpuSet,
}

impl CpuPreference {
    fn discover() -> Option<Self> {
        let allowed = allowed()?;
        let mut capacities = Vec::new();
        for cpu in 0..CPU_COUNT {
            let word = cpu / usize::BITS as usize;
            let bit = 1usize << (cpu % usize::BITS as usize);
            if allowed[word] & bit == 0 {
                continue;
            }
            let capacity: u32 =
                fs::read_to_string(format!("/sys/devices/system/cpu/cpu{cpu}/cpu_capacity"))
                    .ok()?
                    .trim()
                    .parse()
                    .ok()?;
            if capacity == 0 {
                return None;
            }
            capacities.push((cpu, capacity));
        }
        let maximum = capacities.iter().map(|(_, capacity)| *capacity).max()?;
        if capacities.iter().all(|(_, capacity)| *capacity == maximum) {
            return None;
        }
        let mut preferred = [0; CPU_COUNT / usize::BITS as usize];
        for (cpu, capacity) in capacities {
            if capacity == maximum {
                preferred[cpu / usize::BITS as usize] |= 1 << (cpu % usize::BITS as usize);
            }
        }
        Some(Self { preferred })
    }

    fn enter(&self) -> Option<AffinityGuard> {
        let original = allowed()?;
        let preferred = std::array::from_fn(|i| original[i] & self.preferred[i]);
        if preferred == original || preferred.iter().all(|word| *word == 0) || !apply(&preferred) {
            return None;
        }
        Some(AffinityGuard {
            original,
            thread: PhantomData,
        })
    }
}

pub struct AffinityGuard {
    original: CpuSet,
    thread: PhantomData<Rc<()>>,
}

impl Drop for AffinityGuard {
    fn drop(&mut self) {
        apply(&self.original);
    }
}
