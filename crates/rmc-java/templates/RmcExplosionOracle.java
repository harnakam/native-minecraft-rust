package net.minecraft.client.rmc;
import java.util.*;
import io.netty.buffer.Unpooled;
import net.minecraft.network.PacketBuffer;
import net.minecraft.network.play.server.S27PacketExplosion;
import net.minecraft.util.BlockPos;
import net.minecraft.util.Vec3;
/** Captures actual S27 encoding and decoding for signed center/offset cases. */
public final class RmcExplosionOracle {
    public static void main(String[] args) throws Exception {
        float[] centers={-1.75F,-0.75F,0.75F,16777216F};
        for(int i=0;i<centers.length;i++) {
            int center=(int)centers[i];
            S27PacketExplosion packet=new S27PacketExplosion(centers[i],64.25,8.5,2.0F,
                Arrays.asList(new BlockPos(center-2,65,8),new BlockPos(center+127,-64,-120)),new Vec3(0.25,0.5,-0.25));
            PacketBuffer buffer=new PacketBuffer(Unpooled.buffer());
            packet.writePacketData(buffer);
            StringBuilder hex=new StringBuilder("27");
            for(int k=0;k<buffer.readableBytes();k++) hex.append(String.format("%02x",buffer.getUnsignedByte(k)));
            System.out.println("explosion " + i + " wire " + hex);
            S27PacketExplosion decoded=new S27PacketExplosion();decoded.readPacketData(buffer);
            int k=0;
            for(BlockPos pos:decoded.getAffectedBlockPositions()) {
                System.out.println("explosion " + i + " pos" + k++ + " " + pos.getX()+","+pos.getY()+","+pos.getZ());
            }
            buffer.release();
        }
    }
}
