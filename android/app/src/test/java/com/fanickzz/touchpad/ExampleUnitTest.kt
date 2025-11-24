package com.fanickzz.touchpad

import com.google.protobuf.ByteString
import com.google.protobuf.bytesValue
import kotlinx.io.bytestring.buildByteString
import org.junit.Test

import org.junit.Assert.*

import touchpad.v1.HeartbeatOuterClass.Heartbeat
import touchpad.v1.HeartbeatOuterClass.HeartbeatDir

/**
 * Example local unit test, which will execute on the development machine (host).
 *
 * See [testing documentation](http://d.android.com/tools/testing).
 */
class ExampleUnitTest {
    @Test
    fun serialize_and_deserialize() {
        val heartbeat = Heartbeat.newBuilder()
            .setDir(HeartbeatDir.DIR_PING)
            .setSendTs(System.currentTimeMillis())
            .setSeq(0)
            .setCookie(ByteString.copyFromUtf8("hello 世界"))
            .build()
        val bytes = heartbeat.toByteArray()
        val heartbeat2 = Heartbeat.parseFrom(bytes)
        assertEquals(heartbeat, heartbeat2)
    }
}