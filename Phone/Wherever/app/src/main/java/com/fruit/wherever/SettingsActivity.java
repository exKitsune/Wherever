package com.fruit.wherever;

import android.app.Activity;
import android.app.PendingIntent;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Parcelable;
import android.provider.ContactsContract;
import android.text.method.ScrollingMovementMethod;
import android.util.Log;
import android.view.View;
import android.widget.Button;
import android.widget.CompoundButton;
import android.widget.TextView;
import android.widget.ToggleButton;

import androidx.annotation.RequiresApi;
import androidx.appcompat.app.ActionBar;
import androidx.appcompat.app.AppCompatActivity;
import androidx.preference.PreferenceFragmentCompat;

import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;


public class SettingsActivity extends AppCompatActivity {
    private DBManager dbManager = new DBManager(this);

    final String[] from = new String[] { DatabaseHelper.HOST, DatabaseHelper.COMPONENT };
    final String ACTION_APP_OPEN = "com.fruit.wherever.ACTION_APP_OPEN";

    @RequiresApi(api = Build.VERSION_CODES.LOLLIPOP_MR1)
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Intent intent = getIntent();
        Log.e("BRUH", "intent aaaaa" + intent);
        SharedPreferences prefs = getPreferences(Context.MODE_PRIVATE);
        if(intent.getAction().equals(ACTION_APP_OPEN)) {
            Log.e("BRUH", "ACTION_APP_OPEN CALLBACK");
            Log.e("BRUH", intent.toString());
            ComponentName chosen_app = intent.getParcelableExtra(Intent.EXTRA_CHOSEN_COMPONENT);
            Log.e("BRUH", "app: " + chosen_app);
            String url = intent.getStringExtra("url");
            Log.e("BRUH", "url: " + url);

            if(!chosen_app.flattenToString().equals("com.fruit.wherever/com.fruit.wherever.SettingsActivity")) {
                Uri uri = Uri.parse(url);
                String host = uri.getHost();
                dbManager.open();
                Cursor cursor = dbManager.fetch(host);
                if (cursor.getCount() == 0) {
                    dbManager.insert(host, chosen_app.flattenToString());
                } else {
                    dbManager.update(host, chosen_app.flattenToString());
                }
                cursor.close();
                dbManager.close();
            }
            finish();
            return;
        }
        if(intent.getAction() == Intent.ACTION_SEND || intent.getAction() == Intent.ACTION_VIEW) {
            Log.e("BRUH", "ACTION_SEND or ACTION_VIEW");
            Uri uri;
            if(intent.getAction() == Intent.ACTION_SEND && intent.getType() != null) {
                String sharedText = intent.getStringExtra(Intent.EXTRA_TEXT);
                uri = Uri.parse(sharedText);
            } else {
                uri = intent.getData();
            }

            Log.e("BRUH", "URI: " + uri.toString());
            Log.e("BRUH", "URI scheme: \"" + uri.getScheme() + "\"");
            Log.e("BRUH", "URI host: \"" + uri.getHost() + "\"");
            if(uri.getScheme().equals("where")) {
                Log.e("BRUH", "where:// uri");
                String home_ip = uri.getHost();
                int home_port = uri.getPort();

                SharedPreferences.Editor editor = prefs.edit();
                editor.putString("ip", home_ip);
                editor.putInt("port", home_port);
                editor.apply();
                finish();
                return;
            } else { //if(uri.getScheme() == "http" || uri.getScheme() == "https") {
                if (prefs.getBoolean("enabled", false)) {
                    String home_ip = prefs.getString("ip", "192.168.1.11");
                    int home_port = prefs.getInt("port", 8998);

                    if (home_ip == "") {
                        return;
                    }
                    Log.e("BRUH", "ip: " + home_ip + ", port: " + home_port);
                    Runnable r = new Runnable() {
                        @Override
                        public void run() {
                            try {
                                Log.e("BRUH", "I'm gonna send the response");
                                URL url = new URL("http://" + home_ip + ":" + home_port + "/open");
                                HttpURLConnection con = (HttpURLConnection) url.openConnection();
                                con.setDoOutput(true);
                                con.setRequestMethod("POST");
                                con.setRequestProperty("Content-Type", "text/plain; utf-8");
                                try (OutputStream os = con.getOutputStream()) {
                                    byte[] input = uri.toString().getBytes("utf-8");
                                    os.write(input, 0, input.length);
                                }
                                Log.e("BRUH", "HTTP Response: " + con.getResponseCode());
                            } catch (Exception e) {
                                Log.e("BRUH", e.toString());
                            }
                        }
                    };
                    new Thread(r).start();
                } else {
                    if (intent.getAction() != Intent.ACTION_SEND) {
                        //super.onBackPressed();
                        dbManager.open();
                        String host = Uri.parse(intent.getData().toString()).getHost();
                        String component = null;
                        Cursor cursor = dbManager.fetch(host);
                        while (cursor.moveToNext()) {
                            component = cursor.getString(cursor.getColumnIndexOrThrow(DatabaseHelper.COMPONENT));
                        }
                        cursor.close();

                        Log.d("bruh host", host);
                        Log.d("bruh component", component == null ? "null" : component);

                        if (component == null) {
                            Intent sendIntent = new Intent();

                            Log.e("bRUh", "1 "+intent.getAction());
                            Log.e("bRUh", "1 "+intent.getType());
                            Log.e("bRUh", "1 "+intent.getComponent());

                            sendIntent.fillIn(intent, 0);
                            sendIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                            sendIntent.setFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK);
                            sendIntent.setFlags(Intent.FLAG_ACTIVITY_FORWARD_RESULT);
                            Intent receiver = new Intent(this, SettingsActivity.class)
                                    .putExtra("url", intent.getData().toString()).setAction(ACTION_APP_OPEN);
                            PendingIntent pendingIntent = PendingIntent.getActivity(this, 1, receiver, PendingIntent.FLAG_UPDATE_CURRENT);
                            //Intent chooser = Intent.createChooser(sendIntent, "OwO What app would you like to select neko nya~~?", pendingIntent.getIntentSender());

                            String[] blacklist = new String[]{"com.fruit.wherever"};
                            startActivity(generateCustomChooserIntent(sendIntent, blacklist, pendingIntent));
                            //startActivity(chooser);


                        } else {
                            Intent finalIntent = new Intent();
                            finalIntent.fillIn(intent, 0);
                            finalIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                            finalIntent.setFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK);
                            finalIntent.setFlags(Intent.FLAG_ACTIVITY_FORWARD_RESULT);
                            finalIntent.setComponent(ComponentName.unflattenFromString(component));
                            startActivity(finalIntent);
                        }
                        dbManager.close();
                    }
                }
                finish();
                return;
            }
        } else {
            Log.e("BRUH", "bruh settings");
            setContentView(R.layout.settings_activity);
            ActionBar actionBar = getSupportActionBar();
            if (actionBar != null) {
                actionBar.setDisplayHomeAsUpEnabled(true);
            }

            boolean enabled = prefs.getBoolean("enabled", false);

            ToggleButton toggle = (ToggleButton) findViewById(R.id.on_off_button);
            toggle.setChecked(enabled);
            toggle.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
                public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {

                    String home_ip = prefs.getString("ip", "192.168.1.11");
                    int home_port = prefs.getInt("port", 8998);

                    SharedPreferences.Editor editor = prefs.edit();
                    if (isChecked) {
                        editor.putBoolean("enabled", true);
                    } else {
                        editor.putBoolean("enabled", false);
                    }

                    editor.apply();
                }
            });

            Button clickButton = (Button) findViewById(R.id.drop);
            clickButton.setOnClickListener(new View.OnClickListener() {

                @Override
                public void onClick(View v) {
                    dbManager.open();
                    dbManager.drop();
                    dbManager.close();
                }
            });
        }
    }
    @RequiresApi(api = Build.VERSION_CODES.LOLLIPOP_MR1)
    private Intent generateCustomChooserIntent(Intent prototype, String[] forbiddenChoices, PendingIntent pendingIntent) {
        List<Intent> targetedShareIntents = new ArrayList<Intent>();
        List<HashMap<String, String>> intentMetaInfo = new ArrayList<HashMap<String, String>>();
        Intent chooserIntent;

        List<ResolveInfo> resInfo = getPackageManager().queryIntentActivities(prototype, PackageManager.MATCH_ALL);

        if (!resInfo.isEmpty()) {
            for (ResolveInfo resolveInfo : resInfo) {
                if (resolveInfo.activityInfo == null || Arrays.asList(forbiddenChoices).contains(resolveInfo.activityInfo.packageName))
                    continue;

                HashMap<String, String> info = new HashMap<String, String>();
                info.put("packageName", resolveInfo.activityInfo.packageName);
                info.put("className", resolveInfo.activityInfo.name);
                info.put("simpleName", String.valueOf(resolveInfo.activityInfo.loadLabel(getPackageManager())));
                intentMetaInfo.add(info);
            }

            if (!intentMetaInfo.isEmpty()) {
                // sorting for nice readability
                Collections.sort(intentMetaInfo, new Comparator<HashMap<String, String>>() {
                    @Override
                    public int compare(HashMap<String, String> map, HashMap<String, String> map2) {
                        return map.get("simpleName").compareTo(map2.get("simpleName"));
                    }
                });

                // create the custom intent list
                for (HashMap<String, String> metaInfo : intentMetaInfo) {
                    Intent targetedShareIntent = (Intent) prototype.clone();
                    targetedShareIntent.setPackage(metaInfo.get("packageName"));
                    targetedShareIntent.setClassName(metaInfo.get("packageName"), metaInfo.get("className"));
                    targetedShareIntents.add(targetedShareIntent);
                }

                chooserIntent = Intent.createChooser(targetedShareIntents.remove(targetedShareIntents.size() - 1), "Send Link", pendingIntent.getIntentSender());
                chooserIntent.putExtra(Intent.EXTRA_INITIAL_INTENTS, targetedShareIntents.toArray(new Parcelable[]{}));
                return chooserIntent;
            }
        }

        return Intent.createChooser(prototype, "Send Link", pendingIntent.getIntentSender());
    }
}

