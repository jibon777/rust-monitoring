# 🖥️ System Monitor (Rust + Dioxus + Docker)

Aplikasi real-time system monitoring untuk CPU, RAM, Swap, dan Network berbasis **Rust**, **Dioxus 0.6 Fullstack** (Server-Side Rendering + WebAssembly Client), dan siap dijalankan dengan **Docker**.

---

## ✨ Fitur

- **🔲 CPU Monitoring**:
  - Global CPU utilization percentage
  - Informasi nama CPU, core count, dan clock frequency
  - Real-time gauge dan visualisasi penggunaan per-core
- **💾 Memory & Swap**:
  - RAM used, available, total (dalam GB)
  - Swap used dan total
  - Conic-gradient dynamic ring gauge
- **🌐 Network Monitoring**:
  - Real-time download & upload speed (KB/s, MB/s)
  - Total akumulatif data terkirim & diterima (RX / TX)
  - Rincian per-network interface (eth0, wlan0, docker0, dll.)
- **⚡ Dioxus Fullstack Architecture**:
  - Server-Side Rendering (SSR) via Axum + Fullstack RPC Server Functions
  - Client-side WebAssembly (WASM) hydration
  - Real-time polling setiap 2 detik
- **🐳 Docker Ready**:
  - Multi-stage build hemat ukuran (Debian slim runner)
  - Menggunakan `--pid=host` agar membaca metrik fisik host asli

---

## 🚀 Cara Menjalankan dengan Docker

### 1. Menggunakan Docker Compose (Direkomendasikan)

Di direktori project, jalankan:

```bash
docker compose up -d --build
```

Buka browser di:
👉 **[http://localhost:8080](http://localhost:8080)**

Untuk melihat log:
```bash
docker compose logs -f
```

Untuk menghentikan:
```bash
docker compose down
```

---

### 2. Menggunakan Docker CLI Standar

**Build image:**
```bash
docker build -t system-monitor .
```

**Jalankan container (dengan akses host CPU/RAM):**
```bash
docker run -d \
  --name system-monitor \
  --restart unless-stopped \
  --pid=host \
  -p 8080:8080 \
  system-monitor
```

---

## 🛠️ Menjalankan Langsung Tanpa Docker (Local Development)

Jika ingin menjalankan langsung di WSL/Linux tanpa Docker:

```bash
# 1. Pastikan Dioxus CLI terpasang
dx --version

# 2. Jalankan development server (dengan auto-reload)
dx serve
```
Akses dashboard di `http://127.0.0.1:8080`.

# rust-monitoring
