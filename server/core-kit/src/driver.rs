use std::{
    collections::{HashMap, HashSet},
    vec,
};

use anyhow::Result;
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, InputEvent, KeyCode, MiscCode, PropType,
    RelativeAxisCode, SynchronizationCode, UinputAbsSetup, uinput::VirtualDevice,
};
use num_enum::TryFromPrimitive;
use tracing::debug;

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum TouchStatus {
    Down = 1,
    Up = 2,
    Move = 3,
}

/// 触控点数据结构
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub slot: i32,
    pub tracking_id: i32, // -1 表示释放触控点, tracking_id与slot对应
    pub x: i32,
    pub y: i32,
    pub status: TouchStatus,
}

/// 虚拟触摸板驱动
///
/// 只负责发送原始输入事件，不处理任何手势识别逻辑
pub struct Driver {
    device: VirtualDevice,
    width: u32,
    height: u32,
    touched_slots: HashSet<i32>,
    last_input_position: HashMap<i32, (i32, i32)>, // 记录最后输入的原始坐标（用于计算增量）
    last_output_position: HashMap<i32, (i32, i32)>, // 记录最后输出的坐标（应用sensitivity后）
    sensitivity: f32,
    invert_x: bool,
    invert_y: bool,
}

impl Driver {
    /// 创建新的虚拟触摸板驱动
    ///
    /// # Arguments
    /// * `width` - 触摸板宽度（像素）
    /// * `height` - 触摸板高度（像素）
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_MIDDLE);
        keys.insert(KeyCode::BTN_TOUCH);
        keys.insert(KeyCode::BTN_TOOL_FINGER);
        keys.insert(KeyCode::BTN_TOOL_QUINTTAP);
        keys.insert(KeyCode::BTN_TOOL_DOUBLETAP);
        keys.insert(KeyCode::BTN_TOOL_TRIPLETAP);
        keys.insert(KeyCode::BTN_TOOL_QUADTAP);

        // 配置多点触控绝对轴
        let abs_mt_slot = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_MT_SLOT,
            AbsInfo::new(0, 0, 9, 0, 0, 0), // 支持10点触控
        );
        let abs_mt_tracking_id = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_MT_TRACKING_ID,
            AbsInfo::new(0, -1, 65535, 0, 0, 0),
        );
        let abs_mt_x = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_MT_POSITION_X,
            AbsInfo::new(0, 0, width as i32, 0, 0, 0),
        );
        let abs_mt_y = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_MT_POSITION_Y,
            AbsInfo::new(0, 0, height as i32, 0, 0, 0),
        );

        let abs_mt_tool_tip =
            UinputAbsSetup::new(AbsoluteAxisCode::ABS_MT_TOOL_TYPE, AbsInfo::new(0, 0, 2, 0, 0, 0));

        // 配置单点绝对轴（兼容性）
        let abs_x =
            UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, AbsInfo::new(0, 0, width as i32, 0, 0, 0));
        let abs_y = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_Y,
            AbsInfo::new(0, 0, height as i32, 0, 0, 0),
        );

        // 配置相对轴（用于光标移动）
        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_X);
        rel_axes.insert(RelativeAxisCode::REL_Y);

        let mut prop_type_set = AttributeSet::new();
        prop_type_set.insert(PropType::POINTER);
        prop_type_set.insert(PropType::BUTTONPAD);

        let mut misc_types = AttributeSet::new();
        misc_types.insert(MiscCode::MSC_TIMESTAMP);

        // 构建设备
        let device = VirtualDevice::builder()?
            .name("Virtual TouchPad")
            .with_keys(&keys)?
            .with_absolute_axis(&abs_mt_slot)?
            .with_absolute_axis(&abs_mt_tracking_id)?
            .with_absolute_axis(&abs_mt_x)?
            .with_absolute_axis(&abs_mt_y)?
            .with_absolute_axis(&abs_x)?
            .with_absolute_axis(&abs_y)?
            .with_absolute_axis(&abs_mt_tool_tip)?
            .with_relative_axes(&rel_axes)?
            .with_properties(&prop_type_set)?
            .with_msc(&misc_types)?
            .build()?;

        Ok(Self {
            device,
            width,
            height,
            touched_slots: HashSet::new(),
            last_input_position: HashMap::new(),
            last_output_position: HashMap::new(),
            sensitivity: 1.0,
            invert_x: false,
            invert_y: false,
        })
    }

    /// 发送多点触控事件（使用 MT SLOT 协议）
    ///
    /// # Arguments
    /// * `touches` - 触控点切片，每个触控点包含 slot、tracking_id 和坐标
    pub fn emit_multitouch(&mut self, touche_points: &[TouchPoint]) -> Result<()> {
        let old_slots_count = self.touched_slots.len();
        let mut events = Vec::new();
        for point in touche_points {
            events.extend(match point.status {
                TouchStatus::Down => {
                    debug!("Touch down: {:?}", point);
                    self.touched_slots.insert(point.slot);
                    self.emit_point_down(point)
                },
                TouchStatus::Up => {
                    self.touched_slots.remove(&point.slot);
                    self.emit_point_up(point)
                },
                TouchStatus::Move => self.emit_point_move(point),
            });
        }
        let new_slots_count = self.touched_slots.len();
        events.extend(self.get_slot_changed_events(old_slots_count, new_slots_count));
        events.push(InputEvent::new(
            EventType::SYNCHRONIZATION.0,
            SynchronizationCode::SYN_REPORT.0,
            1,
        ));
        self.device.emit(&events)?;
        Ok(())
    }

    pub fn get_slot_changed_events(&self, old_count: usize, new_count: usize) -> Vec<InputEvent> {
        if old_count == new_count {
            return Vec::new();
        }
        let mut events = Vec::new();
        match old_count {
            0 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOUCH.0, 1)),
            1 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_FINGER.0, 0)),
            2 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_DOUBLETAP.0, 0)),
            3 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_TRIPLETAP.0, 0)),
            4 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_QUADTAP.0, 0)),
            5 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_QUINTTAP.0, 0)),
            _ => {},
        }
        match new_count {
            0 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOUCH.0, 0)),
            1 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_FINGER.0, 1)),
            2 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_DOUBLETAP.0, 1)),
            3 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_TRIPLETAP.0, 1)),
            4 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_QUADTAP.0, 1)),
            5 => events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOOL_QUINTTAP.0, 1)),
            _ => {},
        }
        events
    }

    /// 发送单点触控按下事件（使用 MT SLOT 协议）
    pub fn emit_point_down(&mut self, point: &TouchPoint) -> Vec<InputEvent> {
        let tracking_id = point.tracking_id;
        let slot = point.slot;

        // 按下时，输入和输出坐标都初始化为原始坐标
        self.last_input_position.insert(slot, (point.x, point.y));
        self.last_output_position.insert(slot, (point.x, point.y));

        let mut events = vec![
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_SLOT.0, slot),
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_MT_TRACKING_ID.0,
                tracking_id,
            ),
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_MT_POSITION_X.0,
                point.x as i32,
            ),
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_MT_POSITION_Y.0,
                point.y as i32,
            ),
        ];
        events.extend(vec![
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, point.x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, point.y),
        ]);

        events
    }

    /// 发送单点触控抬起事件（使用 MT SLOT 协议）
    pub fn emit_point_up(&mut self, point: &TouchPoint) -> Vec<InputEvent> {
        let slot = point.slot;
        self.last_input_position.remove(&slot);
        self.last_output_position.remove(&slot);
        vec![
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_SLOT.0, slot),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_TRACKING_ID.0, -1),
        ]
    }

    /// 发送单点触控移动事件（使用 MT SLOT 协议）
    pub fn emit_point_move(&mut self, point: &TouchPoint) -> Vec<InputEvent> {
        let slot = point.slot;
        let tracking_id = point.tracking_id;

        // 获取上次的输入坐标（原始坐标）和输出坐标
        let default = (point.x, point.y);
        let (last_input_x, last_input_y) = self.last_input_position.get(&slot).unwrap_or(&default);
        let (last_output_x, last_output_y) =
            self.last_output_position.get(&slot).unwrap_or(&default);

        // 计算输入增量（原始移动距离）
        let delta_x = point.x - last_input_x;
        let delta_y = point.y - last_input_y;

        // 应用 sensitivity 放大移动增量
        let scaled_delta_x = if self.invert_x {
            -delta_x as f32 * self.sensitivity
        } else {
            delta_x as f32 * self.sensitivity
        };
        let scaled_delta_y = if self.invert_y {
            -delta_y as f32 * self.sensitivity
        } else {
            delta_y as f32 * self.sensitivity
        };

        // 计算新的输出坐标 = 上次输出 + 放大后的增量
        let new_output_x = *last_output_x as f32 + scaled_delta_x;
        let new_output_y = *last_output_y as f32 + scaled_delta_y;

        // 不做边界检查，让 evdev 自动处理
        // 这样 last_output_position 可以保存真实的累积坐标，避免到达边界后卡住
        let point_x = new_output_x.round() as i32;
        let point_y = new_output_y.round() as i32;

        // 更新记录（保存真实的累积坐标，不截断）
        self.last_input_position.insert(slot, (point.x, point.y));
        self.last_output_position.insert(slot, (point_x, point_y));
        // 如果是单指触控的话，就不需要重复声明槽了
        let mut events = if self.touched_slots.len() > 1 {
            vec![
                InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_SLOT.0, slot),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_MT_TRACKING_ID.0,
                    tracking_id,
                ),
            ]
        } else {
            vec![]
        };
        events.extend(vec![
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_POSITION_X.0, point_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_POSITION_Y.0, point_y),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, point_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, point_y),
        ]);
        events
    }

    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity;
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn set_invert_x(&mut self, invert_x: bool) {
        self.invert_x = invert_x;
    }

    pub fn set_invert_y(&mut self, invert_y: bool) {
        self.invert_y = invert_y;
    }

    /// 获取触摸板尺寸
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
