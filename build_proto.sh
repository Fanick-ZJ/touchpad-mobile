PROTO_DIR=protos

# 构建rust的proto, 加括号是为了保持在原地
(cd ./server && cargo build -p touchpad-proto)
for file in ${PROTO_DIR}/*.proto; do
    echo "Processing ${file}"
    # 遍历生成安卓的协议文件
    protoc --proto_path=${PROTO_DIR} --kotlin_out=android/protos ${file}
done
