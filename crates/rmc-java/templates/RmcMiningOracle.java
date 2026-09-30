package net.minecraft.client.rmc;

import net.minecraft.block.Block;
import net.minecraft.init.Bootstrap;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.util.BlockPos;
import net.minecraft.enchantment.Enchantment;
import net.minecraft.potion.Potion;
import net.minecraft.potion.PotionEffect;

/** Numeric behavioral observations only; no game resources or reference source export. */
public final class RmcMiningOracle {
    public static void main(String[] args) {
        Bootstrap.register();
        RmcCollisionOracle.QueryWorld world = new RmcCollisionOracle.QueryWorld();
        BlockPos pos = new BlockPos(0, 64, 0);
        int[] tools = {256,257,258,267,268,269,270,271,272,273,274,275,276,277,278,279,283,284,285,286,359};
        for (int id = 0; id <= 197; id++) {
            Block block = Block.getBlockById(id);
            if (block == null) continue;
            world.states.put(pos, block.getDefaultState());
            boolean hand = block.getMaterial().isToolNotRequired();
            System.out.println("mining " + id + " " + block.getBlockHardness(world,pos) + " " + hand + " " + Block.blockRegistry.getNameForObject(block));
            for (int tool : tools) {
                ItemStack stack = new ItemStack(Item.getItemById(tool));
                System.out.println("tool " + id + " " + tool + " " + stack.getStrVsBlock(block) + " " + (hand || stack.canHarvestBlock(block)));
            }
        }
        int[] held = {-1,256,257,258,267,268,269,270,271,272,273,274,275,276,277,278,279,283,284,285,286,359};
        for (String scenario : new String[]{"normal","airborne","efficiency","haste","fatigue","underwater"}) {
            RmcMovementOracle.Player player = new RmcMovementOracle.Player(world);
            player.setPosition(2.5,64.0,0.5);
            player.onGround = !scenario.equals("airborne");
            if(scenario.equals("haste")) player.addPotionEffect(new PotionEffect(Potion.digSpeed.id,1000,1));
            if(scenario.equals("fatigue")) player.addPotionEffect(new PotionEffect(Potion.digSlowdown.id,1000,2));
            world.states.put(new BlockPos(2,65,0),Block.getBlockById(scenario.equals("underwater")?9:0).getDefaultState());
            for(int id=0;id<=197;id++) {
                Block block=Block.getBlockById(id);
                world.states.put(pos,block.getDefaultState());
                for(int tool:held) {
                    ItemStack stack=tool<0?null:new ItemStack(Item.getItemById(tool));
                    if(stack!=null && scenario.equals("efficiency")) stack.addEnchantment(Enchantment.efficiency,2);
                    player.inventory.mainInventory[0]=stack;
                    System.out.println("hardness "+scenario+" "+id+" "+tool+" "+block.getPlayerRelativeBlockHardness(player,world,pos));
                }
            }
        }
    }
}
