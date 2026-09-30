package net.minecraft.client.rmc;
import java.util.Map;
import net.minecraft.init.Bootstrap;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.item.crafting.FurnaceRecipes;
import net.minecraft.tileentity.TileEntityFurnace;

/** Numeric registered smelting and fuel observations, with no resource export. */
public final class RmcFurnacePropertiesOracle {
    public static void main(String[] args) {
        Bootstrap.register();
        for(int id=0;id<32768;id++) {
            Item item=Item.getItemById(id);
            if(item!=null) System.out.println("fuel "+id+" "+TileEntityFurnace.getItemBurnTime(new ItemStack(item)));
        }
        for(Map.Entry<ItemStack,ItemStack> e:FurnaceRecipes.instance().getSmeltingList().entrySet()) {
            ItemStack a=e.getKey(), b=e.getValue();
            System.out.println("smelt "+Item.getIdFromItem(a.getItem())+" "+a.getMetadata()+" "+Item.getIdFromItem(b.getItem())+" "+b.stackSize+" "+b.getMetadata());
        }
    }
}
