package net.minecraft.client.rmc;
import net.minecraft.block.Block;
import net.minecraft.block.state.IBlockState;
import net.minecraft.init.Bootstrap;
import net.minecraft.util.BlockPos;
import net.minecraft.util.MovingObjectPosition;
import net.minecraft.util.Vec3;

/** Numeric selection bounds and real ray hits; resources and Java sources are not exported. */
public final class RmcSelectionOracle {
    public static void main(String[] args) {
        Bootstrap.register();
        RmcCollisionOracle.QueryWorld world=new RmcCollisionOracle.QueryWorld();
        BlockPos pos=new BlockPos(0,64,0);
        double[][] samples={{0.25,0.25},{0.25,0.75},{0.75,0.25},{0.75,0.75},{0.5,0.5}};
        for(int id=0;id<=197;id++) for(int meta=0;meta<16;meta++) {
            world.states.clear();
            Block block=Block.getBlockById(id);
            IBlockState state;
            try {state=block.getStateFromMeta(meta);} catch(RuntimeException error) {continue;}
            world.states.put(pos,state);
            block.setBlockBoundsBasedOnState(world,pos);
            boolean selectable=block.canCollideCheck(state,false);
            System.out.println("select "+id+" "+meta+" "+selectable+" "+block.getBlockBoundsMinX()+","+block.getBlockBoundsMinY()+","+block.getBlockBoundsMinZ()+","+block.getBlockBoundsMaxX()+","+block.getBlockBoundsMaxY()+","+block.getBlockBoundsMaxZ());
            for(int face=0;face<6;face++) for(int sample=0;sample<samples.length;sample++) {
                int axis=face/2;
                double[] start=new double[3],end=new double[3];
                int component=0;
                for(int k=0;k<3;k++) {
                    double base=k==1?64:0;
                    start[k]=base+(k==axis?(face%2==0?-2:2):samples[sample][component++]);
                    end[k]=k==axis?base+(face%2==0?2:-2):start[k];
                }
                Vec3 a=new Vec3(start[0],start[1],start[2]),b=new Vec3(end[0],end[1],end[2]);
                MovingObjectPosition hit=selectable?block.collisionRayTrace(world,pos,a,b):null;
                String value=hit==null?"-":a.distanceTo(hit.hitVec)+","+hit.sideHit.getIndex();
                System.out.println("selectionray "+id+" "+meta+" "+face+" "+sample+" "+value);
            }
        }
    }
}
