PROTO_DIR=protos
# 构建rust的proto, 加括号是为了保持在原地
(cd ./server && cargo build -p touchpad-proto)
