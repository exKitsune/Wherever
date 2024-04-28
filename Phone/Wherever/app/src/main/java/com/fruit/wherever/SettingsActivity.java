package com.fruit.wherever;

import android.app.Activity;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.ActivityNotFoundException;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.database.Cursor;
import android.graphics.Color;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Parcelable;
import android.provider.ContactsContract;
import android.text.method.ScrollingMovementMethod;
import android.util.Log;
import android.util.Pair;
import android.view.View;
import android.widget.AdapterView;
import android.widget.Button;
import android.widget.CompoundButton;
import android.widget.ListView;
import android.widget.SimpleCursorAdapter;
import android.widget.TextView;
import android.widget.Toast;
import android.widget.ToggleButton;

import androidx.annotation.RequiresApi;
import androidx.appcompat.app.ActionBar;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.app.NotificationCompat;
import androidx.core.app.NotificationManagerCompat;
import androidx.preference.PreferenceFragmentCompat;

import org.w3c.dom.Text;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.lang.reflect.Array;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Calendar;
import java.util.Collections;
import java.util.Comparator;
import java.util.Date;
import java.util.HashMap;
import java.util.List;

import static android.app.NotificationChannel.DEFAULT_CHANNEL_ID;


public class SettingsActivity extends AppCompatActivity {
    private DBManager dbManager = new DBManager(this);

    final String[] from = new String[] { DatabaseHelper.HOST, DatabaseHelper.COMPONENT };
    final int[] to = new int[] { R.id.hostTextView, R.id.compTextView };

    final static String ACTION_APP_OPEN = "com.fruit.wherever.ACTION_APP_OPEN";
    final static String ACTION_DEFAULT_SET = "com.fruit.wherever.ACTION_DEFAULT_SET";
    final static String ACTION_TURN_ON = "com.fruit.wherever.ACTION_TURN_ON";

    static Activity c;

    public static boolean getStatus() {
        SharedPreferences prefs = c.getPreferences(Context.MODE_PRIVATE);
        return prefs.getBoolean("enabled", false);
    }

    @RequiresApi(api = Build.VERSION_CODES.O)
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        c = this;
        Intent intent = getIntent();
        Log.e("BRUH", "intent aaaaa" + intent);
        Log.e("BRUH", "LOG INTENT: " + intentToString(intent));
        Log.d("BRUH", "bruh sender" + this.getReferrer().getHost());
        SharedPreferences prefs = getPreferences(Context.MODE_PRIVATE);
        if(intent.getAction() != null && intent.getAction().equals(ACTION_APP_OPEN)) {
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
                Long currentTime = Calendar.getInstance().getTimeInMillis();
                dbManager.put(host, chosen_app.flattenToString(), currentTime);
                dbManager.close();
            }
            finish();
            return;
        }
        if(intent.getAction() != null && intent.getAction().equals(ACTION_DEFAULT_SET)) {
            Log.e("BRUH", "ACTION_DEFAULT_SET CALLBACK");
            ComponentName chosen_app = intent.getParcelableExtra(Intent.EXTRA_CHOSEN_COMPONENT);
            Log.e("New def app", chosen_app.flattenToString());
            if(!chosen_app.flattenToString().equals("com.fruit.wherever/com.fruit.wherever.SettingsActivity")) {
                dbManager.open();
                dbManager.put("DEFAULT_BROWSER", chosen_app.flattenToString(), 0);
                dbManager.close();
            }
            finish();
            return;
        }
        if(intent.getAction() != null && intent.getAction().equals(ACTION_TURN_ON)) {
            SharedPreferences.Editor editor = prefs.edit();
            Log.e("enabling", "" + !prefs.getBoolean("enabled", false));
            editor.putBoolean("enabled", !prefs.getBoolean("enabled", false));
            editor.apply();
            finish();
            return;
        }
        if(intent.getAction() != null && intent.getAction() == Intent.ACTION_SEND || intent.getAction() == Intent.ACTION_VIEW) {
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
                                    byte[] input = uri.toString().getBytes("utf-8");
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

                            if(!good) {
                                Thread thread = new Thread() {
                                    public void run() {
                                        runOnUiThread(new Runnable() {
                                            public void run() {
                                                int duration = Toast.LENGTH_LONG;
                                                Toast toast = Toast.makeText(getApplicationContext(), "Wherever Server Connection Unstable\nTurning OFF", duration);
                                                toast.show();

                                                SharedPreferences.Editor editor = prefs.edit();
                                                editor.putBoolean("enabled", false);
                                                editor.apply();
                                            }
                                        });
                                    }
                                };
                                thread.start();
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
                            sendIntent.addFlags(Intent.FLAG_ACTIVITY_FORWARD_RESULT);
                            Intent receiver = new Intent(this, SettingsActivity.class)
                                    .putExtra("url", intent.getData().toString()).setAction(ACTION_APP_OPEN);
                            PendingIntent pendingIntent = PendingIntent.getActivity(this, 1, receiver, PendingIntent.FLAG_UPDATE_CURRENT);
                            //Intent chooser = Intent.createChooser(sendIntent, "OwO What app would you like to select neko nya~~?", pendingIntent.getIntentSender());

                            List<String> blacklist = new ArrayList<String>();
                            blacklist.add("com.fruit.wherever");
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

                                Pair<Intent, List<Intent>> cci = generateCustomChooserIntent(sendIntent, final_blacklist, pendingIntent, "Send Link");
                                if(cci.second.size() > 1 && !sameComponent) {
                                    startActivity(cci.first);
                                } else {
                                    sendIntent.setComponent(cci.second.get(0).getComponent());
                                    startActivity(sendIntent);
                                }
                            }

                        } else {
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

            ListView listView = (ListView) findViewById(R.id.listView);
            dbManager.open();
            Cursor cursor = dbManager.fetchAll();
            SimpleCursorAdapter adapter = new SimpleCursorAdapter(this, R.layout.listview_row, cursor, from, to, 0);
            adapter.notifyDataSetChanged();

            listView.setAdapter(adapter);

            listView.setOnItemClickListener(new AdapterView.OnItemClickListener() {
                @Override
                public void onItemClick(AdapterView<?> parent, View view, int position, long id) {
                    TextView hTextView = (TextView) view.findViewById(R.id.hostTextView);
                    String host = hTextView.getText().toString();

                    Intent modifyIntent = new Intent(getApplicationContext(), ModifyRecord.class);
                    modifyIntent.putExtra("host", host);

                    startActivity(modifyIntent);
                    finish();
                    return;
                }
            });
            dbManager.close();
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
                    String[] blacklist = new String[]{"com.fruit.wherever", "org.chromium.webview_shell"};
                    Intent receiver = new Intent(SettingsActivity.this, SettingsActivity.class).setAction(ACTION_DEFAULT_SET);
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
                    dbManager.put("POTENTIAL_BROWSERS", potential_browsers, 0);
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

        //List<ResolveInfo> resInfo = getPackageManager().queryIntentActivities(prototype, PackageManager.MATCH_ALL);
        Intent query = new Intent();
        query.setAction(prototype.getAction());
        query.setData(prototype.getData());
        List<ResolveInfo> resInfo = getPackageManager().queryIntentActivities(query, PackageManager.MATCH_ALL);
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

                List<Intent> tSI = new ArrayList<>(targetedShareIntents);
                Log.e("sizeof tsi", String.valueOf(tSI.size()));

                chooserIntent = Intent.createChooser(targetedShareIntents.remove(targetedShareIntents.size() - 1), message, pendingIntent.getIntentSender());
                chooserIntent.putExtra(Intent.EXTRA_INITIAL_INTENTS, targetedShareIntents.toArray(new Parcelable[]{}));

                return new Pair<>(chooserIntent, tSI);
            }
        }

        return new Pair<>(Intent.createChooser(prototype, message, pendingIntent.getIntentSender()), new ArrayList<>());
    }

    public static String intentToString(Intent intent) {
        if (intent == null) {
            return null;
        }

        return intent.toString() + " " + bundleToString(intent.getExtras());
    }

    public static String bundleToString(Bundle bundle) {
        StringBuilder out = new StringBuilder("Bundle[");

        if (bundle == null) {
            out.append("null");
        } else {
            boolean first = true;
            for (String key : bundle.keySet()) {
                if (!first) {
                    out.append(", ");
                }

                out.append(key).append('=');

                Object value = bundle.get(key);

                if (value instanceof int[]) {
                    out.append(Arrays.toString((int[]) value));
                } else if (value instanceof byte[]) {
                    out.append(Arrays.toString((byte[]) value));
                } else if (value instanceof boolean[]) {
                    out.append(Arrays.toString((boolean[]) value));
                } else if (value instanceof short[]) {
                    out.append(Arrays.toString((short[]) value));
                } else if (value instanceof long[]) {
                    out.append(Arrays.toString((long[]) value));
                } else if (value instanceof float[]) {
                    out.append(Arrays.toString((float[]) value));
                } else if (value instanceof double[]) {
                    out.append(Arrays.toString((double[]) value));
                } else if (value instanceof String[]) {
                    out.append(Arrays.toString((String[]) value));
                } else if (value instanceof CharSequence[]) {
                    out.append(Arrays.toString((CharSequence[]) value));
                } else if (value instanceof Parcelable[]) {
                    out.append(Arrays.toString((Parcelable[]) value));
                } else if (value instanceof Bundle) {
                    out.append(bundleToString((Bundle) value));
                } else {
                    out.append(value);
                }

                first = false;
            }
        }

        out.append("]");
        return out.toString();
    }

//    @RequiresApi(api = Build.VERSION_CODES.O)
//    private void logLinks(Context c, String msg) {
//        File path = c.getFilesDir();
//        File logFile = new File(path, "last-fifty-links.txt");
//        int length = (int) logFile.length();
//
//        byte[] bytes = new byte[length];
//
//        try {
//            FileInputStream in = new FileInputStream(logFile);
//            try {
//                in.read(bytes);
//            } finally {
//                in.close();
//            }
//            List<String> contents = Arrays.asList(new String(bytes).split("\n"));
//
//            contents = contents.subList(1, contents.size());
//            contents.add(msg);
//
//            String toFile = String.join("", contents);
//
//            FileOutputStream stream = new FileOutputStream(logFile);
//            try {
//                stream.write(toFile.getBytes());
//            } finally {
//                stream.close();
//            }
//        } catch (IOException e) {
//            Log.e("Wherever", e.toString());
//        }
//    }
}

