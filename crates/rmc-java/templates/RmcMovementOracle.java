package net.minecraft.client.rmc;
import java.util.UUID;
import com.mojang.authlib.GameProfile;
import net.minecraft.block.Block;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraft.init.Bootstrap;
import net.minecraft.util.BlockPos;

/** Exercises actual MCP travel/collision methods in a controlled received world. */
public final class RmcMovementOracle {
    static final class Player extends EntityPlayer {
        Player(RmcCollisionOracle.QueryWorld world) {super(world,new GameProfile(new UUID(0,1),"Oracle"));}
        @Override public boolean isSpectator() {return false;}
        @Override public boolean isServerWorld() {return true;}
    }
    public static void main(String[] args) {
        Bootstrap.register();
        String[] names={"walk","diagonal","sprint","jump","ice","soul_sand","slime","water_depth1","water_depth3","water_depth5","lava_depth3","water","water_jump","shallow_water","flowing_water","water_edge","falling_water","lava","ladder","web","slab"};
        for(String name:names) {
            RmcCollisionOracle.QueryWorld world=new RmcCollisionOracle.QueryWorld();
            int floor=name.equals("ice")?79:name.equals("soul_sand")?88:name.equals("slime")?165:1;
            for(int x=-32;x<64;x++) for(int z=-32;z<64;z++) {
                world.states.put(new BlockPos(x,63,z),Block.getBlockById(floor).getStateFromMeta(0));
                if(name.equals("water")||name.equals("lava")||name.startsWith("water_depth")||name.equals("lava_depth3")) for(int y=64;y<66;y++) world.states.put(new BlockPos(x,y,z),Block.getBlockById(name.startsWith("water")?9:11).getStateFromMeta(0));
                if(name.equals("water_jump")||name.equals("shallow_water")||name.equals("flowing_water")) {
                    int top=name.equals("shallow_water")?65:66;
                    int level=name.equals("flowing_water")?Math.max(0,Math.min(7,z-8)):0;
                    for(int y=64;y<top;y++) world.states.put(new BlockPos(x,y,z),Block.getBlockById(9).getStateFromMeta(level));
                }
            }
            if(name.equals("ladder")) for(int y=64;y<80;y++) {
                world.states.put(new BlockPos(8,y,8),Block.getBlockById(65).getStateFromMeta(5));
                world.states.put(new BlockPos(9,y,8),Block.getBlockById(1).getStateFromMeta(0));
            }
            if(name.equals("water_edge")) for(int x=0;x<16;x++) for(int z=0;z<=8;z++) world.states.put(new BlockPos(x,64,z),Block.getBlockById(9).getStateFromMeta(0));
            if(name.equals("water_edge")) for(int x=0;x<16;x++) world.states.put(new BlockPos(x,64,9),Block.getBlockById(1).getStateFromMeta(0));
            if(name.equals("falling_water")) for(int y=64;y<70;y++) {
                world.states.put(new BlockPos(8,y,8),Block.getBlockById(9).getStateFromMeta(8));
                world.states.put(new BlockPos(9,y,8),Block.getBlockById(1).getStateFromMeta(0));
            }
            if(name.equals("web")) for(int y=64;y<66;y++) for(int z=8;z<20;z++) world.states.put(new BlockPos(8,y,z),Block.getBlockById(30).getStateFromMeta(0));
            if(name.equals("slab")) for(int x=0;x<16;x++) {
                world.states.put(new BlockPos(x,64,9),Block.getBlockById(44).getStateFromMeta(0));
                world.states.put(new BlockPos(x,64,10),Block.getBlockById(1).getStateFromMeta(0));
            }
            Player player=new Player(world);
            player.setPosition(8.5,name.equals("soul_sand")?63.875:64.0,8.5);player.onGround=true;
            if(name.equals("slime")) {player.setPosition(8.5,68,8.5);player.onGround=false;player.motionY=-0.4;}
            if(name.contains("depth")) {
                net.minecraft.item.ItemStack boots=new net.minecraft.item.ItemStack(net.minecraft.init.Items.diamond_boots);
                boots.addEnchantment(net.minecraft.enchantment.Enchantment.depthStrider, Integer.parseInt(name.substring(name.length()-1)));
                player.inventory.armorInventory[0]=boots;
            }
            player.rotationYaw=name.equals("diagonal")?37.25F:name.equals("sprint")||name.equals("ladder")?-90F:0F;
            for(int tick=1;tick<=80;tick++) {
                player.moveForward=1.0F;
                player.moveStrafing=name.equals("diagonal")?1.0F:0.0F;
                player.setSprinting(name.equals("sprint"));
                player.setJumping(name.equals("jump")||name.equals("water_jump")||name.equals("water_edge"));
                player.handleWaterMovement();
                player.onLivingUpdate();
                System.out.println("travel "+name+" "+tick+" "+player.posX+" "+player.posY+" "+player.posZ+" "+player.motionX+" "+player.motionY+" "+player.motionZ+" "+player.onGround);
            }
        }
    }
}
