package com.fruit.wherever;

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
import android.util.Log;
import android.util.Pair;
import android.widget.Toast;

import androidx.annotation.RequiresApi;
import androidx.appcompat.app.AppCompatActivity;

import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Base64;
import java.util.Calendar;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;

import static com.fruit.wherever.SettingsActivity.ACTION_APP_OPEN;
import static com.fruit.wherever.SettingsActivity.ACTION_DEFAULT_SET;

public class LinkActivity extends AppCompatActivity {
    private DBManager dbManager = new DBManager(this);

    @RequiresApi(api = Build.VERSION_CODES.O)
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Log.e("BRUH", "bruh links");

        Intent intent = getIntent();
        SharedPreferences prefs = SettingsActivity.getSharedPreferences(this.getApplicationContext());

        if(intent.getAction().equals(ACTION_APP_OPEN)) {
            Log.e("BRUH", "ACTION_APP_OPEN CALLBACK");
            Log.e("BRUH", intent.toString());
            ComponentName chosen_app = intent.getParcelableExtra(Intent.EXTRA_CHOSEN_COMPONENT);
            Log.e("BRUH", "app: " + chosen_app);
            String url = intent.getStringExtra("url");
            Log.e("BRUH", "url: " + url);

            //make sure we aren't recursively calling our app on accident
            //we put host, component, last accessed time in
            if(!chosen_app.flattenToString().equals("com.fruit.wherever/com.fruit.wherever.LinkActivity")) {
                Uri uri = Uri.parse(url);
                String host = uri.getHost();
                dbManager.open();
                Long currentTime = Calendar.getInstance().getTimeInMillis();
                dbManager.put(host, chosen_app.flattenToString(), currentTime);
                dbManager.close();
            }
        }
        //callback when choosing a default browser
        if(intent.getAction().equals(ACTION_DEFAULT_SET)) {
            Log.e("BRUH", "ACTION_DEFAULT_SET CALLBACK");
            ComponentName chosen_app = intent.getParcelableExtra(Intent.EXTRA_CHOSEN_COMPONENT);
            Log.e("New def app", chosen_app.flattenToString());
            if(!chosen_app.flattenToString().equals("com.fruit.wherever/com.fruit.wherever.LinkActivity")) {
                dbManager.open();
                dbManager.put("DEFAULT_BROWSER", chosen_app.flattenToString(), 0);
                dbManager.close();
            }
        }

        //primary portion of the app
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
                Log.e("BRUH", "where://" + uri);
                String home_ip = uri.getHost();
                int home_port = uri.getPort();
                String server_pub_key_b64 = uri.getFragment();

                SharedPreferences.Editor editor = prefs.edit();
                editor.putString("ip", home_ip);
                editor.putInt("port", home_port);
                editor.putString("server_pub_key", server_pub_key_b64);
                if(prefs.getString("client_key", "null").equals("null")) {
                    byte[] generated_key = WhereverCrypto.genKey();
                    String generated_key_b64 = Base64.getEncoder().encodeToString(generated_key);
                    editor.putString("client_key", generated_key_b64);
                }
                editor.apply();
            } else { //if(uri.getScheme() == "http" || uri.getScheme() == "https") {
                if (prefs.getBoolean("enabled", false)) {
                    String home_ip = prefs.getString("ip", "192.168.1.11");
                    int home_port = prefs.getInt("port", 8998);

                    if (home_ip == "") {
                        return;
                    }
                    Log.e("BRUH", "ip: " + home_ip + ", port: " + home_port);
                    //Encrypt our message
                    String server_pub_key_b64 = prefs.getString("server_pub_key", "null");
                    if(server_pub_key_b64.equals("null")) {
                        Toast.makeText(getApplicationContext(), "Wherever Server Public Key Error\nTurning OFF", Toast.LENGTH_LONG).show();

                        SharedPreferences.Editor editor = prefs.edit();
                        editor.putBoolean("enabled", false);
                        editor.apply();
                        finish();
                        return;
                    }
                    byte[] server_pub_key = Base64.getDecoder().decode(server_pub_key_b64);
                    String client_key_b64 = prefs.getString("client_key", "null");
                    byte[] client_key = Base64.getDecoder().decode(client_key_b64);
                    byte[] encrypted_msg = WhereverCrypto.encMsg(uri.toString(), client_key, server_pub_key);

                    final byte[] input = encrypted_msg;

                    //create async thread to send packets to server
                    Runnable r = new Runnable() {
                        @Override
                        public void run() {
                            boolean good = true;
                            try {
                                Log.e("BRUH", "I'm gonna send the response");
                                URL url = new URL("http://" + home_ip + ":" + home_port + "/open");
                                HttpURLConnection con = (HttpURLConnection) url.openConnection();
                                con.setDoOutput(true);
                                con.setRequestMethod("POST");
                                con.setRequestProperty("Content-Type", "text/plain; utf-8");
                                con.setConnectTimeout(5000);
                                try (OutputStream os = con.getOutputStream()) {
                                    os.write(input, 0, input.length);
                                }
                                int rc = con.getResponseCode();
                                if(rc != 200) {
                                    good = false;
                                }
                                Log.e("BRUH", "HTTP Response: " + rc);
                            } catch (Exception e) {
                                Log.e("BRUH", e.toString());
                                good = false;
                            }

                            //send toast in main thread to notify user of success
                            // or failure, in which case turn app off
                            final boolean f_good = good;
                            runOnUiThread(new Runnable() {
                                public void run() {
                                    if(!f_good) {
                                        Toast.makeText(getApplicationContext(), "Wherever Server Connection Unstable\nTurning OFF", Toast.LENGTH_LONG).show();

                                        SharedPreferences.Editor editor = prefs.edit();
                                        editor.putBoolean("enabled", false);
                                        editor.apply();
                                    } else {
                                        Toast.makeText(getApplicationContext(), "Link Sent", Toast.LENGTH_SHORT).show();
                                    }
                                }
                            });
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
                        boolean sameComponent = false;
                        if (component != null) {
                            sameComponent = component.split("/")[0].equals(this.getReferrer().getHost());
                        }
                        if ((component == null) || sameComponent) {
                            Intent sendIntent = new Intent();

                            Log.e("bRUh", "1 "+intent.getAction());
                            Log.e("bRUh", "1 "+intent.getType());
                            Log.e("bRUh", "1 "+intent.getComponent());

                            sendIntent.fillIn(intent, 0);
                            //Intent.FLAG_ACTIVITY_FORWARD_RESULT is needed to forward the intention such as open in custom tab
                            sendIntent.addFlags(Intent.FLAG_ACTIVITY_FORWARD_RESULT);
                            //store url in an extra so during callback we can know url/host + component clicked
                            Intent receiver = new Intent(this, LinkActivity.class)
                                    .putExtra("url", intent.getData().toString()).setAction(ACTION_APP_OPEN);
                            PendingIntent pendingIntent = PendingIntent.getActivity(this, 1, receiver, PendingIntent.FLAG_UPDATE_CURRENT);

                            List<String> blacklist = new ArrayList<String>();
                            blacklist.add("com.fruit.wherever");
                            blacklist.add("org.chromium.webview_shell");
                            blacklist.add(this.getReferrer().getHost());

                            String default_browser = null;
                            ComponentName default_browser_full = null;
                            String potential_browsers = null;

                            cursor = dbManager.fetch("DEFAULT_BROWSER");

                            while (cursor.moveToNext()) {
                                default_browser_full = ComponentName.unflattenFromString(cursor.getString(cursor.getColumnIndexOrThrow(DatabaseHelper.COMPONENT)));
                                default_browser = default_browser_full.getPackageName();
                            }
                            cursor.close();
                            if (sameComponent) { // If we find the intent is going back to the app that sent it, send to default browser instead
                                sendIntent.setComponent(default_browser_full);
                                startActivity(sendIntent);
                            } else {
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

                                //Log.e("def browser", default_browser);

                                String[] final_blacklist = blacklist.toArray(new String[blacklist.size()]);
                                for (String s : final_blacklist) {
                                    Log.e("f black", s);
                                }

                                Pair<Intent, List<Intent>> cci = generateCustomChooserIntent(getApplicationContext(), sendIntent, final_blacklist, pendingIntent, "Send Link");
                                if(cci.second.size() > 1 && !sameComponent) {
                                    startActivity(cci.first);
                                } else {
                                    sendIntent.setComponent(cci.second.get(0).getComponent());
                                    startActivity(sendIntent);
                                }
                            }

                        } else {
                            //If a component was found for that specific host, simply launch it and update db with last accessed time
                            Intent finalIntent = new Intent();
                            finalIntent.fillIn(intent, 0);
                            finalIntent.addFlags(Intent.FLAG_ACTIVITY_FORWARD_RESULT);
                            finalIntent.setComponent(ComponentName.unflattenFromString(component));
                            Long currentTime = Calendar.getInstance().getTimeInMillis();
                            dbManager.put(host, component, currentTime);
                            startActivity(finalIntent);
                        }
                        dbManager.close();
                    }
                }
            }
        }
        finish();
    }

    //Adapted from https://gist.github.com/mediavrog/5625602
    @RequiresApi(api = Build.VERSION_CODES.LOLLIPOP_MR1)
    static public Pair<Intent, List<Intent>> generateCustomChooserIntent(Context ctxt, Intent prototype, String[] forbiddenChoices, PendingIntent pendingIntent, String message) {
        List<Intent> targetedShareIntents = new ArrayList<Intent>();
        List<HashMap<String, String>> intentMetaInfo = new ArrayList<HashMap<String, String>>();
        Intent chooserIntent;

        //List<ResolveInfo> resInfo = getPackageManager().queryIntentActivities(prototype, PackageManager.MATCH_ALL);
        Intent query = new Intent();
        query.setAction(prototype.getAction());
        query.setData(prototype.getData());
        List<ResolveInfo> resInfo = ctxt.getPackageManager().queryIntentActivities(query, PackageManager.MATCH_ALL);
        Log.e("size of res", String.valueOf(resInfo.size()));

        for(ResolveInfo res : resInfo) {
            Log.e("res", res.activityInfo.packageName);
        }
        if (!resInfo.isEmpty()) {
            for (ResolveInfo resolveInfo : resInfo) {
                if (resolveInfo.activityInfo == null || Arrays.asList(forbiddenChoices).contains(resolveInfo.activityInfo.packageName))
                    continue;

                HashMap<String, String> info = new HashMap<String, String>();
                info.put("packageName", resolveInfo.activityInfo.packageName);
                info.put("className", resolveInfo.activityInfo.name);
                info.put("simpleName", String.valueOf(resolveInfo.activityInfo.loadLabel(ctxt.getPackageManager())));
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

                List<Intent> tSI = new ArrayList<>(targetedShareIntents);
                Log.e("sizeof tsi", String.valueOf(tSI.size()));

                chooserIntent = Intent.createChooser(targetedShareIntents.remove(targetedShareIntents.size() - 1), message, pendingIntent.getIntentSender());
                chooserIntent.putExtra(Intent.EXTRA_INITIAL_INTENTS, targetedShareIntents.toArray(new Parcelable[]{}));

                return new Pair<>(chooserIntent, tSI);
            }
        }

        return new Pair<>(Intent.createChooser(prototype, message, pendingIntent.getIntentSender()), new ArrayList<>());
    }
}
