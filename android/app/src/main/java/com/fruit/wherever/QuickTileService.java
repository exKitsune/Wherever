package com.fruit.wherever;

import android.app.Application;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.drawable.Icon;
import android.os.Build;
import android.service.quicksettings.Tile;
import android.service.quicksettings.TileService;
import android.util.Log;

import androidx.annotation.DrawableRes;
import androidx.annotation.RequiresApi;

@RequiresApi(api = Build.VERSION_CODES.N)
public class QuickTileService extends TileService {
    public QuickTileService() {}

    @Override
    public void onCreate() {
    }

    @Override
    public void onStartListening() {
        super.onStartListening();
        updateTile();
    }

    @RequiresApi(api = Build.VERSION_CODES.N)
    @Override
    public void onClick() {
        SharedPreferences prefs = SettingsActivity.getSharedPreferences(QuickTileService.this.getApplicationContext());
        boolean enabled = prefs.getBoolean("enabled", false);
        SharedPreferences.Editor editor = prefs.edit();
        Log.e("enabling", "" + !enabled);
        editor.putBoolean("enabled", !enabled);
        editor.apply();

        updateTile();
//        Intent intent = new Intent(this, SettingsActivity.class);
//        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
//        intent.setAction(SettingsActivity.ACTION_TURN_ON);
//        startActivityAndCollapse(intent);
    }

    @RequiresApi(api = Build.VERSION_CODES.N)
    private void updateTile() {
        Tile tile = getQsTile();

        SharedPreferences prefs = SettingsActivity.getSharedPreferences(QuickTileService.this.getApplicationContext());
        boolean enabled = prefs.getBoolean("enabled", false);

        @DrawableRes int icon = R.drawable.ic_tile;
        tile.setState(Tile.STATE_INACTIVE);
        if(enabled) {
            tile.setState(Tile.STATE_ACTIVE);
        }
        tile.setIcon(Icon.createWithResource(getApplicationContext(), icon));
        tile.updateTile();
    }
}
