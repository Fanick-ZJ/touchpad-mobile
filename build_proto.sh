PROTO_DIR=protos
# 构建rust的proto, 加括号是为了保持在原地
cargo clean -p touchpad-proto
cargo build -p touchpad-proto
