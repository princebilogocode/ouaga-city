//! Small UDP multiplayer: one player hosts (port 47777), the others join with
//! the host's IP address. The host relays every player's position to everyone.

use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

pub const PORT: u16 = 47777;

#[derive(Clone, Debug)]
pub struct Remote {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub kind: u8, // 0 on foot, 1 car, 2 moto, 3 taxi, 4 police, 5 minibus
    pub color: [u8; 3],
    pub wanted: u8,
    pub seen: Instant,
}

pub struct Net {
    sock: UdpSocket,
    host: bool,
    server: Option<SocketAddr>,
    peers: Vec<(SocketAddr, Instant)>,
    pub remotes: HashMap<String, Remote>,
    pub id: String,
    last_send: Instant,
    pub status: String,
}

pub fn local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".into())
}

fn clean(s: &str) -> String {
    s.replace('|', " ").chars().take(16).collect()
}

impl Net {
    pub fn host() -> Result<Net, String> {
        let sock = UdpSocket::bind(("0.0.0.0", PORT)).map_err(|e| format!("Port {PORT} occupé : {e}"))?;
        sock.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Net::new(sock, true, None, format!("Hôte sur {}:{PORT}", local_ip())))
    }
    pub fn join(addr: &str) -> Result<Net, String> {
        let a = if addr.contains(':') { addr.to_string() } else { format!("{addr}:{PORT}") };
        let server = a
            .to_socket_addrs()
            .map_err(|e| format!("Adresse invalide : {e}"))?
            .find(|s| s.is_ipv4())
            .ok_or("Adresse invalide")?;
        let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        sock.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Net::new(sock, false, Some(server), format!("Connexion à {server}…")))
    }
    fn new(sock: UdpSocket, host: bool, server: Option<SocketAddr>, status: String) -> Net {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        Net {
            sock,
            host,
            server,
            peers: Vec::new(),
            remotes: HashMap::new(),
            id: format!("{:x}", t.as_nanos() as u64 ^ std::process::id() as u64),
            last_send: Instant::now() - Duration::from_secs(1),
            status,
        }
    }
    pub fn is_host(&self) -> bool {
        self.host
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(&mut self, name: &str, x: f32, y: f32, angle: f32, kind: u8, color: [u8; 3], wanted: u8) {
        let mut buf = [0u8; 512];
        while let Ok((n, from)) = self.sock.recv_from(&mut buf) {
            let msg = String::from_utf8_lossy(&buf[..n]).to_string();
            if let Some(r) = parse(&msg) {
                if r.0 != self.id {
                    self.remotes.insert(r.0.clone(), r.1);
                }
                if self.host {
                    match self.peers.iter_mut().find(|p| p.0 == from) {
                        Some(p) => p.1 = Instant::now(),
                        None => self.peers.push((from, Instant::now())),
                    }
                    for (p, _) in &self.peers {
                        if *p != from {
                            let _ = self.sock.send_to(msg.as_bytes(), p);
                        }
                    }
                }
            }
        }
        self.peers.retain(|p| p.1.elapsed() < Duration::from_secs(5));
        self.remotes.retain(|_, r| r.seen.elapsed() < Duration::from_secs(4));
        if self.last_send.elapsed() >= Duration::from_millis(50) {
            self.last_send = Instant::now();
            let msg = format!(
                "OC1|{}|{}|{:.1}|{:.1}|{:.3}|{}|{}|{}|{}|{}",
                self.id, clean(name), x, y, angle, kind, color[0], color[1], color[2], wanted
            );
            if self.host {
                for (p, _) in &self.peers {
                    let _ = self.sock.send_to(msg.as_bytes(), p);
                }
            } else if let Some(s) = self.server {
                let _ = self.sock.send_to(msg.as_bytes(), s);
            }
        }
        let n = self.remotes.len();
        if self.host {
            self.status = format!("Hôte {}:{PORT} · {} joueur(s) connecté(s)", local_ip(), n);
        } else if let Some(s) = self.server {
            self.status = if n > 0 { format!("Connecté à {s} · {} autre(s) joueur(s)", n) } else { format!("En attente de l'hôte {s}…") };
        }
    }
}

fn parse(m: &str) -> Option<(String, Remote)> {
    let p: Vec<&str> = m.split('|').collect();
    if p.len() != 11 || p[0] != "OC1" {
        return None;
    }
    Some((
        p[1].to_string(),
        Remote {
            name: p[2].to_string(),
            x: p[3].parse().ok()?,
            y: p[4].parse().ok()?,
            angle: p[5].parse().ok()?,
            kind: p[6].parse().ok()?,
            color: [p[7].parse().ok()?, p[8].parse().ok()?, p[9].parse().ok()?],
            wanted: p[10].parse().ok()?,
            seen: Instant::now(),
        },
    ))
}
