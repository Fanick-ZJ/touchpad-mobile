PROTO_DIR=protos
ANDROID_PROTO_DIR=android/app/src/main/java/

mkdir -p "$ANDROID_PROTO_DIR"

# 构建rust的proto, 加括号是为了保持在原地
(cd ./server && cargo build -p touchpad-proto)
for file in ${PROTO_DIR}/*.proto; do
    echo "Processing ${file}"
    # 遍历生成安卓的协议文件
    protoc \
    --proto_path=${PROTO_DIR} \
    --java_out=${ANDROID_PROTO_DIR} \
    --kotlin_out=${ANDROID_PROTO_DIR} \
    ${file}
done
