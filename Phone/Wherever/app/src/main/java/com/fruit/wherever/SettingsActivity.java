package com.fruit.wherever;

import android.app.PendingIntent;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.text.method.ScrollingMovementMethod;
import android.util.Log;
import android.view.View;
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

public class SettingsActivity extends AppCompatActivity {

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
                    if(intent.getAction() != Intent.ACTION_SEND) {
                        Intent sendIntent = new Intent();
                        sendIntent.fillIn(intent, 0);

                        Intent receiver = new Intent(this, SettingsActivity.class)
                                .putExtra("url", intent.getData().toString()).setAction(ACTION_APP_OPEN);
                        PendingIntent pendingIntent = PendingIntent.getActivity(this, 1, receiver, PendingIntent.FLAG_UPDATE_CURRENT);
                        Intent chooser = Intent.createChooser(sendIntent, "OwO What app would you like to select neko nya~~?", pendingIntent.getIntentSender());
                        startActivity(chooser);
                    }
                }
                finish();
                super.onBackPressed();
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
        }
    }
}

