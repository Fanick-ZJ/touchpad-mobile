/**
 * 触摸事件类型
 */
const TouchEventTypes = {
  START: "start",
  END: "end",
  MOVE: "move",
  CANCEL: "cancel",
  JOINED: "joined", //新的触控点加入
  LEFT: "left", //触控点离开
} as const;

/**
 * 触摸状态
 */
const TouchStatus = {
  STARTED: "started", //触控正式开始
  MOVING: "moving", //触控移动中
  ENDED: "ended", //触控结束
} as const;

/**
 * 设备信息
 * 对应 Rust 后端的 DiscoverDevice 结构
 * @see src-tauri/src/state.rs:DiscoverDevice
 */
export interface DiscoverDevice {
  /** 设备名称 */
  name: string;
  /** 设备 IP 地址 */
  address: string;
  /** 设备完整名称 (mDNS fullname) */
  fullName: string;
  /** 登录端口 */
  loginPort: number;
  /** 后端端口 */
  backendPort: number;
}

/**
 * IPC 事件名称定义
 * 与 Rust 后端的 emit 事件保持一致
 * @see src-tauri/src/emit.rs
 */
export const IPCEvents = {
  /** 设备发现事件 */
  FOUND_DEVICE: "found-device",
  /** 设备登录事件 */
  DEVICE_LOGIN: "device-login",
  /** 设备离线事件 */
  DEVICE_OFFLINE: "device-offline",
  /** 连接成功事件 */
  CONNECTION_SUCCESS: "connection-success",
  /** 连接失败事件 */
  CONNECTION_ERROR: "connection-error",
} as const;

/**
 * IPC 事件载荷类型映射
 * 用于类型安全的事件监听
 */
export interface IPCEventPayloads {
  [IPCEvents.FOUND_DEVICE]: DiscoverDevice;
  [IPCEvents.DEVICE_LOGIN]: DiscoverDevice;
  [IPCEvents.DEVICE_OFFLINE]: string;
  [IPCEvents.CONNECTION_SUCCESS]: void;
  [IPCEvents.CONNECTION_ERROR]: string;
}

/**
 * 类型安全的事件监听辅助函数
 * 确保事件名称和载荷类型匹配
 */
export type EventListener<Event extends keyof IPCEventPayloads> = (
  payload: IPCEventPayloads[Event]
) => void | Promise<void>;

export { TouchEventTypes, TouchStatus };

