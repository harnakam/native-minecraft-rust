package net.minecraft.client.rmc;

import java.util.*;
import net.minecraft.block.Block;
import net.minecraft.block.state.IBlockState;
import net.minecraft.client.multiplayer.WorldClient;
import net.minecraft.init.Bootstrap;
import net.minecraft.init.Blocks;
import net.minecraft.item.Item;
import net.minecraft.world.chunk.Chunk;
import net.minecraft.profiler.Profiler;
import net.minecraft.util.AxisAlignedBB;
import net.minecraft.util.BlockPos;
import net.minecraft.world.*;

/** Local behavioral oracle. Prints numbers, never source or game assets. */
public final class RmcCollisionOracle {
    static class QueryWorld extends WorldClient {
        final Map<BlockPos,IBlockState> states = new HashMap<BlockPos,IBlockState>();
        final Map<Long,Chunk> chunks = new HashMap<Long,Chunk>();
        @Override protected boolean isChunkLoaded(int x,int z,boolean allowEmpty) { return true; }
        @Override public Chunk getChunkFromChunkCoords(int x,int z) {
            long key=((long)x<<32) ^ (z&0xffffffffL);
            Chunk chunk=chunks==null ? null : chunks.get(key);
            if(chunk==null) {chunk=new Chunk(this,x,z);chunk.onChunkLoad();if(chunks!=null) chunks.put(key,chunk);}
            return chunk;
        }
        QueryWorld() { super(null, new WorldSettings(0L, WorldSettings.GameType.SURVIVAL, false, false, WorldType.DEFAULT), 0, EnumDifficulty.NORMAL, new Profiler()); }
        @Override public IBlockState getBlockState(BlockPos pos) {
            IBlockState state = states == null ? null : states.get(pos);
            return state == null ? Blocks.air.getDefaultState() : state;
        }
        @Override public boolean isAirBlock(BlockPos pos) { return getBlockState(pos).getBlock() == Blocks.air; }
    }
    public static void main(String[] args) {
        Bootstrap.register();
        for (int id=0;id<4096;id++) {
            Item item=Item.getItemById(id);
            if (item!=null) System.out.println("item "+id+" "+item.getItemStackLimit());
        }
        QueryWorld world = new QueryWorld();
        BlockPos origin = new BlockPos(0,64,0);
        AxisAlignedBB mask = new AxisAlignedBB(-10,0,-10,10,256,10);
        for (int id=0;id<=197;id++) for(int meta=0;meta<16;meta++) {
            world.states.clear();
            try {
                Block block = Block.getBlockById(id);
                if (block == null) continue;
                IBlockState state = block.getStateFromMeta(meta);
                world.states.put(origin,state);
                block.setBlockBoundsBasedOnState(world,origin);
                List<AxisAlignedBB> boxes = new ArrayList<AxisAlignedBB>();
                block.addCollisionBoxesToList(world,origin,state,mask,boxes,null);
                StringBuilder line = new StringBuilder("shape ").append(id).append(' ').append(meta);
                for (AxisAlignedBB box:boxes) line.append(' ').append(box.minX).append(',').append(box.minY-64).append(',').append(box.minZ).append(',').append(box.maxX).append(',').append(box.maxY-64).append(',').append(box.maxZ);
                System.out.println(line);
            } catch (RuntimeException error) {
                System.out.println("invalid "+id+" "+meta+" "+error.getClass().getSimpleName());
            }
        }
    }
}
