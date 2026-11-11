#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SystemMetrics {
    pub hostname: String,
    pub os_name: String,
    pub kernel_version: String,
    pub uptime_secs: u64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub network: NetworkMetrics,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CpuMetrics {
    pub name: String,
    pub core_count: usize,
    pub total_usage: f32,
    pub per_core_usage: Vec<f32>,
    pub frequency_mhz: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_usage_percent: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NetworkMetrics {
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

// ============================================================================
// Server Functions — only the body runs on the server (native target)
// ============================================================================

#[server]
async fn get_system_metrics() -> Result<SystemMetrics, ServerFnError> {
    use sysinfo::{Networks, System};
    use std::time::Duration;

    // Refresh networks and CPU
    let mut networks = Networks::new_with_refreshed_list();
    let mut sys = System::new_all();
    sys.refresh_cpu_all();
    tokio::time::sleep(Duration::from_millis(200)).await;
    sys.refresh_all();
    networks.refresh(true);

    // ── CPU ──────────────────────────────────────────────────────────────
    let cpus = sys.cpus();
    let cpu = CpuMetrics {
        name: cpus
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown".into()),
        core_count: cpus.len(),
        total_usage: sys.global_cpu_usage(),
        per_core_usage: cpus.iter().map(|c| c.cpu_usage()).collect(),
        frequency_mhz: cpus.first().map(|c| c.frequency()).unwrap_or(0),
    };

    // ── Memory ──────────────────────────────────────────────────────────
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let swap_total = sys.total_swap();
    let swap_used = sys.used_swap();

    let memory = MemoryMetrics {
        total_bytes: total_mem,
        used_bytes: used_mem,
        available_bytes: sys.available_memory(),
        usage_percent: if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        },
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
        swap_usage_percent: if swap_total > 0 {
            (swap_used as f32 / swap_total as f32) * 100.0
        } else {
            0.0
        },
    };

    // ── Network ─────────────────────────────────────────────────────────
    let mut total_rx_speed: u64 = 0;
    let mut total_tx_speed: u64 = 0;
    let mut total_rx: u64 = 0;
    let mut total_tx: u64 = 0;
    let mut interfaces = Vec::new();

    for (name, data) in &networks {
        let rx_speed = data.received().saturating_mul(5);
        let tx_speed = data.transmitted().saturating_mul(5);
        let rx_tot = data.total_received();
        let tx_tot = data.total_transmitted();

        total_rx_speed = total_rx_speed.saturating_add(rx_speed);
        total_tx_speed = total_tx_speed.saturating_add(tx_speed);
        total_rx = total_rx.saturating_add(rx_tot);
        total_tx = total_tx.saturating_add(tx_tot);

        interfaces.push(NetworkInterface {
            name: name.clone(),
            rx_bytes_per_sec: rx_speed,
            tx_bytes_per_sec: tx_speed,
            total_rx_bytes: rx_tot,
            total_tx_bytes: tx_tot,
        });
    }

    // Sort active interfaces first, put loopback last
    interfaces.sort_by(|a, b| {
        let a_lo = a.name.starts_with("lo");
        let b_lo = b.name.starts_with("lo");
        match (a_lo, b_lo) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => (b.rx_bytes_per_sec + b.tx_bytes_per_sec).cmp(&(a.rx_bytes_per_sec + a.tx_bytes_per_sec)),
        }
    });

    let network = NetworkMetrics {
        rx_bytes_per_sec: total_rx_speed,
        tx_bytes_per_sec: total_tx_speed,
        total_rx_bytes: total_rx,
        total_tx_bytes: total_tx,
        interfaces,
    };

    Ok(SystemMetrics {
        hostname: System::host_name().unwrap_or_else(|| "Unknown".into()),
        os_name: System::name().unwrap_or_else(|| "Unknown".into()),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".into()),
        uptime_secs: System::uptime(),
        cpu,
        memory,
        network,
    })
}

// ============================================================================
// Entry Point
// ============================================================================

fn main() {
    launch(App);
}

// ============================================================================
// Platform-agnostic async sleep
// ============================================================================

async fn async_sleep(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(ms).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
}

// ============================================================================
// Root Component
// ============================================================================

#[component]
fn App() -> Element {
    let mut metrics = use_signal(|| None::<SystemMetrics>);
    let mut connected = use_signal(|| true);

    // Poll the server for fresh metrics every 2 s
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            match get_system_metrics().await {
                Ok(data) => {
                    metrics.set(Some(data));
                    connected.set(true);
                }
                Err(_) => {
                    connected.set(false);
                }
            }
            async_sleep(2000).await;
        }
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        document::Link {
            rel: "icon",
            href: "data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 100 100%22><text y=%22.9em%22 font-size=%2290%22>🖥️</text></svg>"
        }

        div { class: "dashboard",
            // ── Header ──────────────────────────────────────────────────
            header { class: "header",
                div { class: "header-content",
                    h1 { "🖥️ System Monitor" }
                    p { class: "subtitle", "Real-time resource monitoring" }
                }
                div {
                    class: if connected() { "status-badge live" } else { "status-badge offline" },
                    div { class: "pulse-dot" }
                    span { if connected() { "Live" } else { "Offline" } }
                }
            }

            // ── Body ────────────────────────────────────────────────────
            match metrics() {
                Some(m) => rsx! {
                    SystemInfoBar { metrics: m.clone() }
                    div { class: "metrics-grid",
                        CpuCard { cpu: m.cpu.clone() }
                        MemoryCard { memory: m.memory.clone() }
                    }
                    div { class: "metrics-grid single",
                        NetworkCard { network: m.network.clone() }
                    }
                },
                None => rsx! {
                    div { class: "loading",
                        div { class: "spinner" }
                        p { "Loading system metrics…" }
                    }
                },
            }

            // ── Footer ─────────────────────────────────────────────────
            footer { class: "footer",
                p { "System Monitor • Built with Rust + Dioxus • Running in Docker 🐳" }
            }
        }
    }
}

// ============================================================================
// System Info Bar
// ============================================================================

#[component]
fn SystemInfoBar(metrics: SystemMetrics) -> Element {
    let uptime = format_uptime(metrics.uptime_secs);

    rsx! {
        div { class: "card info-bar",
            InfoChip { icon: "🏠", label: "Host",   value: metrics.hostname }
            InfoChip { icon: "🐧", label: "OS",     value: metrics.os_name }
            InfoChip { icon: "⚙️",  label: "Kernel", value: metrics.kernel_version }
            InfoChip { icon: "⏱️",  label: "Uptime", value: uptime }
        }
    }
}

#[component]
fn InfoChip(icon: String, label: String, value: String) -> Element {
    rsx! {
        div { class: "info-chip",
            span { class: "chip-icon", "{icon}" }
            div { class: "chip-text",
                span { class: "chip-label", "{label}" }
                span { class: "chip-value", "{value}" }
            }
        }
    }
}

// ============================================================================
// CPU Card
// ============================================================================

#[component]
fn CpuCard(cpu: CpuMetrics) -> Element {
    let cls = color_class(cpu.total_usage);

    rsx! {
        div { class: "card metric-card",
            div { class: "card-header",
                h2 { "🔲 CPU" }
                span { class: "badge {cls}", "{cpu.total_usage:.1}%" }
            }
            p { class: "metric-subtitle", "{cpu.name}" }
            p { class: "metric-detail", "{cpu.core_count} cores · {cpu.frequency_mhz} MHz" }

            div { class: "gauge-row",
                RingGauge { value: cpu.total_usage, label: "Total".to_string() }
            }

            h3 { class: "section-title", "Per-Core Usage" }
            div { class: "core-grid",
                for (i, usage) in cpu.per_core_usage.iter().enumerate() {
                    CoreBar { index: i, usage: *usage }
                }
            }
        }
    }
}

#[component]
fn CoreBar(index: usize, usage: f32) -> Element {
    let cls = color_class(usage);
    rsx! {
        div { class: "core-item",
            div { class: "core-head",
                span { class: "core-label", "C{index}" }
                span { class: "core-pct {cls}", "{usage:.0}%" }
            }
            div { class: "progress-bar",
                div { class: "progress-fill {cls}", style: "width:{usage}%" }
            }
        }
    }
}

// ============================================================================
// Memory Card
// ============================================================================

#[component]
fn MemoryCard(memory: MemoryMetrics) -> Element {
    let cls = color_class(memory.usage_percent);
    let used  = gb(memory.used_bytes);
    let avail = gb(memory.available_bytes);
    let total = gb(memory.total_bytes);
    let sw_used  = gb(memory.swap_used_bytes);
    let sw_total = gb(memory.swap_total_bytes);
    let sw_cls = color_class(memory.swap_usage_percent);

    rsx! {
        div { class: "card metric-card",
            div { class: "card-header",
                h2 { "💾 Memory" }
                span { class: "badge {cls}", "{memory.usage_percent:.1}%" }
            }

            div { class: "gauge-row",
                RingGauge { value: memory.usage_percent, label: "RAM".to_string() }
            }

            div { class: "detail-list",
                DetailRow { label: "Used".to_string(),      value: format!("{used:.2} GB") }
                DetailRow { label: "Available".to_string(),  value: format!("{avail:.2} GB") }
                DetailRow { label: "Total".to_string(),      value: format!("{total:.2} GB") }
            }

            if memory.swap_total_bytes > 0 {
                h3 { class: "section-title", "Swap" }
                div { class: "progress-bar large",
                    div { class: "progress-fill {sw_cls}", style: "width:{memory.swap_usage_percent}%" }
                }
                p { class: "swap-text",
                    "{sw_used:.2} / {sw_total:.2} GB ({memory.swap_usage_percent:.1}%)"
                }
            }
        }
    }
}

// ============================================================================
// Network Card
// ============================================================================

#[component]
fn NetworkCard(network: NetworkMetrics) -> Element {
    let down_speed = format_speed(network.rx_bytes_per_sec);
    let up_speed = format_speed(network.tx_bytes_per_sec);
    let total_down = format_bytes(network.total_rx_bytes);
    let total_up = format_bytes(network.total_tx_bytes);

    rsx! {
        div { class: "card metric-card network-card",
            div { class: "card-header",
                h2 { "🌐 Network" }
                div { class: "net-badges",
                    span { class: "badge normal", "⬇ {down_speed}" }
                    span { class: "badge warning", "⬆ {up_speed}" }
                }
            }
            p { class: "metric-subtitle", "Total: ⬇ {total_down} · ⬆ {total_up}" }

            div { class: "net-interface-list",
                for iface in network.interfaces.iter() {
                    div { class: "net-interface-item",
                        div { class: "net-iface-head",
                            span { class: "net-iface-name", "🔌 {iface.name}" }
                            span { class: "net-iface-speed", "⬇ {format_speed(iface.rx_bytes_per_sec)} · ⬆ {format_speed(iface.tx_bytes_per_sec)}" }
                        }
                        div { class: "net-iface-totals",
                            span { "Cumulative: ⬇ {format_bytes(iface.total_rx_bytes)} / ⬆ {format_bytes(iface.total_tx_bytes)}" }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Shared UI Components
// ============================================================================

/// CSS conic-gradient ring gauge (no SVG needed).
#[component]
fn RingGauge(value: f32, label: String) -> Element {
    let v = value.clamp(0.0, 100.0);
    let deg = (v / 100.0) * 360.0;
    let cls = color_class(v);

    rsx! {
        div { class: "ring-gauge",
            div {
                class: "ring-outer {cls}",
                style: "background:conic-gradient(var(--gauge-color) {deg}deg, var(--gauge-track) {deg}deg)",
                div { class: "ring-inner",
                    span { class: "ring-value", "{v:.1}" }
                    span { class: "ring-unit", "%" }
                }
            }
            span { class: "ring-label", "{label}" }
        }
    }
}

#[component]
fn DetailRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "detail-row",
            span { class: "detail-label", "{label}" }
            span { class: "detail-value", "{value}" }
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn color_class(pct: f32) -> &'static str {
    if pct > 90.0 {
        "critical"
    } else if pct > 70.0 {
        "warning"
    } else {
        "normal"
    }
}

fn format_uptime(s: u64) -> String {
    let d = s / 86400;
    let h = (s % 86400) / 3600;
    let m = (s % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

fn gb(bytes: u64) -> f64 {
    bytes as f64 / 1_073_741_824.0
}

fn format_speed(bytes_per_sec: u64) -> String {
    let f = bytes_per_sec as f64;
    if f >= 1_048_576.0 {
        format!("{:.1} MB/s", f / 1_048_576.0)
    } else if f >= 1_024.0 {
        format!("{:.1} KB/s", f / 1_024.0)
    } else {
        format!("{bytes_per_sec} B/s")
    }
}

fn format_bytes(bytes: u64) -> String {
    let f = bytes as f64;
    if f >= 1_073_741_824.0 {
        format!("{:.2} GB", f / 1_073_741_824.0)
    } else if f >= 1_048_576.0 {
        format!("{:.1} MB", f / 1_048_576.0)
    } else if f >= 1_024.0 {
        format!("{:.1} KB", f / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}

