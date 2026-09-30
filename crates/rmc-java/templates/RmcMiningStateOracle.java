package net.minecraft.client.rmc;

import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import com.mojang.authlib.GameProfile;
import net.minecraft.block.Block;
import net.minecraft.block.state.IBlockState;
import net.minecraft.client.Minecraft;
import net.minecraft.client.audio.ISound;
import net.minecraft.client.audio.SoundHandler;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.multiplayer.PlayerControllerMP;
import net.minecraft.client.network.NetHandlerPlayClient;
import net.minecraft.init.Bootstrap;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.network.Packet;
import net.minecraft.network.play.client.C07PacketPlayerDigging;
import net.minecraft.stats.StatFileWriter;
import net.minecraft.util.BlockPos;
import net.minecraft.util.EnumFacing;
import net.minecraft.world.WorldSettings;
import sun.misc.Unsafe;

/** Calls the actual controller. Only audio, GL bootstrap and transport are replaced by test adapters. */
public final class RmcMiningStateOracle {
    static final class QuietSound extends SoundHandler {
        QuietSound() {super(null,null);}
        @Override public void playSound(ISound sound) {}
    }
    static final class ProbeMinecraft extends Minecraft {
        QuietSound sound;
        ProbeMinecraft() {super(null);}
        @Override public SoundHandler getSoundHandler() {return sound;}
    }
    static final class World extends RmcCollisionOracle.QueryWorld {
        @Override public boolean setBlockState(BlockPos pos,IBlockState state,int flags) {states.put(pos,state);return true;}
    }
    static final class Network extends NetHandlerPlayClient {
        final List<String> events = new ArrayList<String>();
        Network(Minecraft mc) {super(mc,null,null,new GameProfile(new UUID(0,1),"MiningOracle"));}
        @Override public void addToSendQueue(Packet packet) {
            if(packet instanceof C07PacketPlayerDigging) {
                C07PacketPlayerDigging dig=(C07PacketPlayerDigging)packet;
                events.add(dig.getStatus().ordinal()+","+dig.getPosition().getX()+","+dig.getPosition().getY()+","+dig.getPosition().getZ()+","+dig.getFacing().getIndex());
            }
        }
    }
    public static void main(String[] args) throws Exception {
        Bootstrap.register();
        Field access=Unsafe.class.getDeclaredField("theUnsafe");access.setAccessible(true);
        Unsafe allocator=(Unsafe)access.get(null);
        Field damage=PlayerControllerMP.class.getDeclaredField("curBlockDamageMP");damage.setAccessible(true);
        Field hitting=PlayerControllerMP.class.getDeclaredField("isHittingBlock");hitting.setAccessible(true);
        for(String scenario:new String[]{"stone","switch","release","bedrock","slime","creative","creative_sword"}) {
            ProbeMinecraft mc=(ProbeMinecraft)allocator.allocateInstance(ProbeMinecraft.class);
            mc.sound=(QuietSound)allocator.allocateInstance(QuietSound.class);
            World world=new World();
            Network network=new Network(mc);
            mc.theWorld=world;
            mc.thePlayer=new EntityPlayerSP(mc,world,network,new StatFileWriter());
            mc.thePlayer.setPosition(2.5,64,0.5);mc.thePlayer.onGround=true;
            int block=scenario.equals("bedrock")?7:scenario.equals("slime")?165:1;
            world.states.put(new BlockPos(0,64,0),Block.getBlockById(block).getDefaultState());
            world.states.put(new BlockPos(1,64,0),Block.getBlockById(block).getDefaultState());
            mc.thePlayer.inventory.mainInventory[0]=new ItemStack(Item.getItemById(scenario.equals("creative_sword")?276:278));
            PlayerControllerMP controller=new PlayerControllerMP(mc,network);
            controller.setGameType(scenario.startsWith("creative")?WorldSettings.GameType.CREATIVE:WorldSettings.GameType.SURVIVAL);
            for(int tick=1;tick<=20;tick++) {
                BlockPos pos=new BlockPos(scenario.equals("switch") && tick>=4?1:0,64,0);
                boolean held=!scenario.equals("release") || tick<4;
                if(tick==1) controller.clickBlock(pos,EnumFacing.UP);
                if(held && !world.isAirBlock(pos)) controller.onPlayerDamageBlock(pos,EnumFacing.UP);
                else controller.resetBlockRemoving();
                String events=network.events.isEmpty()?"-":String.join(";",network.events);
                network.events.clear();
                System.out.println("minestate "+scenario+" "+tick+" "+damage.getFloat(controller)+" "+hitting.getBoolean(controller)+" "+world.isAirBlock(pos)+" "+events);
            }
        }
    }
}
