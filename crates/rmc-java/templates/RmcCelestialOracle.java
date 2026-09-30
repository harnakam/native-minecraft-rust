package net.minecraft.client.rmc;
import net.minecraft.world.WorldProviderSurface;
public final class RmcCelestialOracle {
    public static void main(String[] args) {
        WorldProviderSurface provider=new WorldProviderSurface();
        for(long time=0;time<24000;time++) for(float partial:new float[]{0,.25F,.5F,.75F}) {
            System.out.println("celestial "+time+" "+partial+" "+Float.floatToIntBits(provider.calculateCelestialAngle(time,partial))+" "+provider.getMoonPhase(time));
        }
    }
}
