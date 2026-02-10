//! 实时触控延迟测试模块
//!
//! 提供轻量级的实时延迟计算和统计功能

use std::collections::VecDeque;

/// 实时延迟统计器
pub struct RealtimeLatencyTracker {
    /// 最近N个延迟样本（用于计算移动平均）
    samples: VecDeque<u64>,
    /// 最大样本数
    max_samples: usize,
    /// 时钟偏移（手机时间 - 服务器时间，单位：毫秒）
    clock_offset_ms: i64,
    /// 期望的下一个序列号
    expected_seq: u32,
    /// 总数据包数
    total_packets: u64,
    /// 丢包数
    lost_packets: u64,
    /// 最小延迟
    min_latency: Option<u64>,
    /// 最大延迟
    max_latency: Option<u64>,
}

impl RealtimeLatencyTracker {
    /// 创建新的延迟跟踪器
    pub fn new(window_size: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(window_size),
            max_samples: window_size,
            clock_offset_ms: 0,
            expected_seq: 0,
            total_packets: 0,
            lost_packets: 0,
            min_latency: None,
            max_latency: None,
        }
    }

    /// 设置时钟偏移（用于同步手机和服务器时间）
    pub fn set_clock_offset(&mut self, offset_ms: i64) {
        self.clock_offset_ms = offset_ms;
    }

    /// 重置统计数据
    pub fn reset(&mut self) {
        self.samples.clear();
        self.expected_seq = 0;
        self.total_packets = 0;
        self.lost_packets = 0;
        self.min_latency = None;
        self.max_latency = None;
    }

    /// 记录一个触控数据包的延迟
    ///
    /// # 参数
    /// - `seq`: 数据包序列号
    /// - `phone_ts_ms`: 手机生成时间戳（毫秒）
    /// - `server_ts_us`: 服务器接收时间戳（微秒）
    ///
    /// # 返回
    /// - `Option<RealtimeLatencyData>`: 当前延迟数据，如果计算失败返回 None
    pub fn record_packet(
        &mut self,
        seq: u32,
        phone_ts_ms: i64,
        server_ts_us: u64,
    ) -> Option<RealtimeLatencyData> {
        self.total_packets += 1;

        // 检测丢包
        if self.expected_seq > 0 && seq > self.expected_seq {
            self.lost_packets += (seq - self.expected_seq) as u64;
        }
        self.expected_seq = seq + 1;

        // 计算延迟（考虑时钟偏移）
        // 手机时间转微秒 - 时钟偏移转微秒
        let phone_ts_us = (phone_ts_ms * 1000) as i64 - (self.clock_offset_ms * 1000);
        let latency_us = server_ts_us as i64 - phone_ts_us;

        if latency_us < 0 {
            // 时钟未同步或延迟为负，忽略
            return None;
        }

        let latency_us = latency_us as u64;

        // 更新最小/最大延迟
        if self.min_latency.is_none() || latency_us < self.min_latency.unwrap() {
            self.min_latency = Some(latency_us);
        }
        if self.max_latency.is_none() || latency_us > self.max_latency.unwrap() {
            self.max_latency = Some(latency_us);
        }

        // 添加到滑动窗口
        self.samples.push_back(latency_us);
        if self.samples.len() > self.max_samples {
            self.samples.pop_front();
        }

        // 计算移动平均
        let avg_latency: u64 = if self.samples.is_empty() {
            0
        } else {
            self.samples.iter().sum::<u64>() / self.samples.len() as u64
        };

        // 计算丢包率
        let packet_loss_rate = if self.total_packets > 0 {
            (self.lost_packets as f64 / self.total_packets as f64) * 100.0
        } else {
            0.0
        };

        Some(RealtimeLatencyData {
            current_latency_us: latency_us,
            avg_latency_us: avg_latency,
            min_latency_us: self.min_latency.unwrap_or(0),
            max_latency_us: self.max_latency.unwrap_or(0),
            packet_loss_rate,
            total_packets: self.total_packets,
            seq,
        })
    }

    /// 获取当前统计数据（不记录新数据）
    pub fn get_current_stats(&self) -> RealtimeLatencyData {
        let avg_latency: u64 = if self.samples.is_empty() {
            0
        } else {
            self.samples.iter().sum::<u64>() / self.samples.len() as u64
        };

        let packet_loss_rate = if self.total_packets > 0 {
            (self.lost_packets as f64 / self.total_packets as f64) * 100.0
        } else {
            0.0
        };

        RealtimeLatencyData {
            current_latency_us: self.samples.back().copied().unwrap_or(0),
            avg_latency_us: avg_latency,
            min_latency_us: self.min_latency.unwrap_or(0),
            max_latency_us: self.max_latency.unwrap_or(0),
            packet_loss_rate,
            total_packets: self.total_packets,
            seq: self.expected_seq.saturating_sub(1),
        }
    }
}

/// 实时延迟数据
#[derive(Debug, Clone, Copy)]
pub struct RealtimeLatencyData {
    /// 当前数据包延迟（微秒）
    pub current_latency_us: u64,
    /// 平均延迟（微秒，基于滑动窗口）
    pub avg_latency_us: u64,
    /// 最小延迟（微秒）
    pub min_latency_us: u64,
    /// 最大延迟（微秒）
    pub max_latency_us: u64,
    /// 丢包率（百分比）
    pub packet_loss_rate: f64,
    /// 总数据包数
    pub total_packets: u64,
    /// 序列号
    pub seq: u32,
}

impl RealtimeLatencyData {
    /// 转换为更友好的显示格式
    pub fn to_display(&self) -> LatencyDisplay {
        LatencyDisplay {
            current_ms: self.current_latency_us as f64 / 1000.0,
            avg_ms: self.avg_latency_us as f64 / 1000.0,
            min_ms: self.min_latency_us as f64 / 1000.0,
            max_ms: self.max_latency_us as f64 / 1000.0,
            packet_loss_percent: self.packet_loss_rate,
            total_packets: self.total_packets,
        }
    }
}

/// 延迟显示数据（前端友好的格式）
#[derive(Debug, Clone, serde::Serialize)]
pub struct LatencyDisplay {
    /// 当前延迟（毫秒）
    pub current_ms: f64,
    /// 平均延迟（毫秒）
    pub avg_ms: f64,
    /// 最小延迟（毫秒）
    pub min_ms: f64,
    /// 最大延迟（毫秒）
    pub max_ms: f64,
    /// 丢包率（百分比）
    pub packet_loss_percent: f64,
    /// 总数据包数
    pub total_packets: u64,
}
