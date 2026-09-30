package net.minecraft.client.rmc;

import net.minecraft.init.Bootstrap;
import net.minecraft.item.Item;

/** Authored registry observations; exports numeric properties, never resources. */
public final class RmcItemPropertiesOracle {
    public static void main(String[] args) {
        Bootstrap.register();
        for (int id = 0; id < 32768; id++) {
            Item item = Item.getItemById(id);
            if (item != null) {
                System.out.println("item_property " + id + " " + item.getItemStackLimit()
                    + " " + item.getHasSubtypes() + " " + item.getMaxDamage());
            }
        }
    }
}
