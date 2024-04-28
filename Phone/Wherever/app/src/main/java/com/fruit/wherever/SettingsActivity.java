package com.fruit.wherever;

import android.app.Activity;
import android.app.PendingIntent;
import android.content.ActivityNotFoundException;
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
import android.util.Pair;
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
import java.lang.reflect.Array;
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
    final String ACTION_DEFAULT_SET = "com.fruit.wherever.ACTION_DEFAULT_SET";

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
                dbManager.put(host, chosen_app.flattenToString());
                dbManager.close();
            }
            finish();
            return;
        }
        if(intent.getAction().equals(ACTION_DEFAULT_SET)) {
            Log.e("BRUH", "ACTION_DEFAULT_SET CALLBACK");
            ComponentName chosen_app = intent.getParcelableExtra(Intent.EXTRA_CHOSEN_COMPONENT);
            String def_app = intent.getStringExtra("url");
            if(!chosen_app.flattenToString().equals("com.fruit.wherever/com.fruit.wherever.SettingsActivity")) {
                dbManager.open();
                dbManager.put(def_app, chosen_app.flattenToString());
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
                            sendIntent.addFlags(Intent.FLAG_ACTIVITY_REQUIRE_NON_BROWSER | Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_FORWARD_RESULT);
                            Intent receiver = new Intent(this, SettingsActivity.class)
                                    .putExtra("url", intent.getData().toString()).setAction(ACTION_APP_OPEN);
                            PendingIntent pendingIntent = PendingIntent.getActivity(this, 1, receiver, PendingIntent.FLAG_UPDATE_CURRENT);
                            //Intent chooser = Intent.createChooser(sendIntent, "OwO What app would you like to select neko nya~~?", pendingIntent.getIntentSender());

                            List<String> blacklist = new ArrayList<String>();
                            blacklist.add("com.fruit.wherever");
                            blacklist.add("org.chromium.webview_shell");

                            String default_browser = null;
                            String potential_browsers = null;

                            cursor = dbManager.fetch("DEFAULT_BROWSER");

                            while (cursor.moveToNext()) {
                                default_browser = cursor.getString(cursor.getColumnIndexOrThrow(DatabaseHelper.COMPONENT)).split("/")[0];
                            }
                            cursor.close();
                            cursor = dbManager.fetch("POTENTIAL_BROWSERS");
                            while (cursor.moveToNext()) {
                                potential_browsers = cursor.getString(cursor.getColumnIndexOrThrow(DatabaseHelper.COMPONENT));
                            }
                            cursor.close();
                            if(potential_browsers != null) {
                                List<String> pbList = Arrays.asList(potential_browsers.split(","));
                                for(String s : pbList) {
                                    s = s.split("/")[0];
                                    if (!s.equals(default_browser)) {
                                        blacklist.add(s);
                                    }
                                }
                            }

                            Log.e("def browser", default_browser);

                            String[] final_blacklist = blacklist.toArray(new String[blacklist.size()]);
                            for (String s : final_blacklist) {
                                Log.e("f black", s);
                            }

                            startActivity(generateCustomChooserIntent(sendIntent, final_blacklist, pendingIntent, "Send Link").first);

                        } else {
                            Intent finalIntent = new Intent();
                            finalIntent.fillIn(intent, 0);
                            finalIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_FORWARD_RESULT);
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

            TextView textView = (TextView) findViewById(R.id.conn_info);

            String home_ip = prefs.getString("ip", "192.168.1.11");
            int home_port = prefs.getInt("port", 8998);

            textView.setText("Current Server: " + home_ip + ":" + home_port);

            ToggleButton toggle = (ToggleButton) findViewById(R.id.on_off_button);
            toggle.setChecked(enabled);
            toggle.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
                public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {
                    SharedPreferences.Editor editor = prefs.edit();
                    if (isChecked) {
                        editor.putBoolean("enabled", true);
                    } else {
                        editor.putBoolean("enabled", false);
                    }

                    editor.apply();
                }
            });

            Button defaultButton = (Button) findViewById(R.id.set_default);
            defaultButton.setOnClickListener((new View.OnClickListener() {
                @RequiresApi(api = Build.VERSION_CODES.O)
                @Override
                public void onClick(View v) {
                    Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse("https://example.com"));
                    Intent sendIntent = new Intent();
                    sendIntent.fillIn(intent, 0);
                    String[] blacklist = new String[]{"com.fruit.wherever"};
                    Intent receiver = new Intent(SettingsActivity.this, SettingsActivity.class)
                            .putExtra("url", "DEFAULT_BROWSER").setAction(ACTION_DEFAULT_SET);
                    PendingIntent pendingIntent = PendingIntent.getActivity(SettingsActivity.this, 1, receiver, PendingIntent.FLAG_UPDATE_CURRENT);
                    Pair<Intent, List<Intent>> cci = generateCustomChooserIntent(sendIntent, blacklist, pendingIntent, "Choose a default browser");
                    String potential_browsers = "";
                    List<String> pbList = new ArrayList<>();
                    for (Intent c : cci.second) {
                        Log.d("intents", c.getComponent().toString());
                        pbList.add(c.getComponent().flattenToString());
                    }
                    potential_browsers = String.join(",", pbList);
                    dbManager.open();
                    dbManager.put("POTENTIAL_BROWSERS", potential_browsers);
                    dbManager.close();
                    startActivity(cci.first);
                }
            }));

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
    private Pair<Intent, List<Intent>> generateCustomChooserIntent(Intent prototype, String[] forbiddenChoices, PendingIntent pendingIntent, String message) {
        List<Intent> targetedShareIntents = new ArrayList<Intent>();
        List<HashMap<String, String>> intentMetaInfo = new ArrayList<HashMap<String, String>>();
        Intent chooserIntent;

        List<ResolveInfo> resInfo = getPackageManager().queryIntentActivities(prototype, PackageManager.MATCH_ALL);
        Log.e("size of res", String.valueOf(resInfo.size()));
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

                List<Intent> tSI = targetedShareIntents;
                Log.e("sizeof tsi", String.valueOf(tSI.size()));
                chooserIntent = Intent.createChooser(targetedShareIntents.remove(targetedShareIntents.size() - 1), message, pendingIntent.getIntentSender());
                chooserIntent.putExtra(Intent.EXTRA_INITIAL_INTENTS, targetedShareIntents.toArray(new Parcelable[]{}));
                return new Pair<>(chooserIntent, tSI);
            }
        }

        return new Pair<>(Intent.createChooser(prototype, message, pendingIntent.getIntentSender()), new ArrayList<>());
    }
}

