package net.minecraft.client.rmc;
import java.util.*;
import net.minecraft.block.Block;
import net.minecraft.block.state.IBlockState;
import net.minecraft.block.material.Material;
import net.minecraft.init.Bootstrap;
import net.minecraft.util.AxisAlignedBB;
import net.minecraft.util.BlockPos;
/** Tests connected geometry against every registered neighboring block type. */
public final class RmcNeighborOracle {
    public static void main(String[] args) {
        Bootstrap.register();
        for(int id=0;id<=197;id++) {
            Block b=Block.getBlockById(id);
            System.out.println("connect " + id + " " + (b.isFullCube() && b.getMaterial().isOpaque() && b.getMaterial()!=Material.gourd && id!=166) + " " + b.isFullBlock());
        }
        RmcCollisionOracle.QueryWorld world=new RmcCollisionOracle.QueryWorld();
        BlockPos pos=new BlockPos(0,64,0);
        int[][] directions={{-1,0},{1,0},{0,-1},{0,1}};
        for(int target:new int[]{85,113,188,189,190,191,192,101,102,160,139})
        for(int neighbor=0;neighbor<=197;neighbor++) for(int mask=0;mask<16;mask++) {
            world.states.clear();
            Block block=Block.getBlockById(target);
            IBlockState state=block.getStateFromMeta(0);
            world.states.put(pos,state);
            for(int side=0;side<4;side++) if((mask & (1 << side))!=0)
                world.states.put(pos.add(directions[side][0],0,directions[side][1]),Block.getBlockById(neighbor).getStateFromMeta(0));
            List<AxisAlignedBB> boxes=new ArrayList<AxisAlignedBB>();
            block.addCollisionBoxesToList(world,pos,state,new AxisAlignedBB(-10,0,-10,10,256,10),boxes,null);
            StringBuilder line=new StringBuilder("shape ").append(target*10000+neighbor*16+mask).append(" 0");
            for(AxisAlignedBB b:boxes) line.append(' ').append(b.minX).append(',').append(b.minY-64).append(',').append(b.minZ).append(',').append(b.maxX).append(',').append(b.maxY-64).append(',').append(b.maxZ);
            System.out.println(line);
            block.setBlockBoundsBasedOnState(world,pos);
            System.out.println("shape " + (target*10000+neighbor*16+mask) + " 1 " + block.getBlockBoundsMinX()+","+block.getBlockBoundsMinY()+","+block.getBlockBoundsMinZ()+","+block.getBlockBoundsMaxX()+","+block.getBlockBoundsMaxY()+","+block.getBlockBoundsMaxZ());
        }
    }
}
