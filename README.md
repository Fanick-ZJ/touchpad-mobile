# 服务端启动测试
```bash
cargo run -p cli -- --config server/cli/tests/config.yml
```
# 安卓端日志输出监控
```bash
adb logcat | grep -E "touchpad_mobile_lib"
```
