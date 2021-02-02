package com.fruit.wherever;

import android.app.PendingIntent;
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
        SharedPreferences prefs = getPreferences(Context.MODE_PRIVATE);
        if(intent.getAction() == ACTION_APP_OPEN) {
            String chosen_app = intent.getStringExtra(Intent.EXTRA_CHOSEN_COMPONENT);
            String url = intent.getStringExtra("url");
            return;
        }
        if(intent.getAction() == Intent.ACTION_SEND || intent.getAction() == Intent.ACTION_VIEW) {
            Uri uri;
            if(intent.getAction() == Intent.ACTION_SEND && intent.getType() != null) {
                String sharedText = intent.getStringExtra(Intent.EXTRA_TEXT);
                uri = Uri.parse(sharedText);
            } else {
                uri = intent.getData();
            }

            if(uri.getScheme() == "where") {
                String home_ip = uri.getHost();
                int home_port = uri.getPort();

                SharedPreferences.Editor editor = prefs.edit();
                editor.putString("ip", home_ip);
                editor.putInt("port", home_port);
                editor.apply();
            } else { //if(uri.getScheme() == "http" || uri.getScheme() == "https") {
                if (prefs.getBoolean("enabled", false)) {
                    String home_ip = prefs.getString("ip", "192.168.1.11");
                    int home_port = prefs.getInt("port", 8998);

                    if (home_ip == "") {
                        return;
                    }
                    Runnable r = new Runnable() {
                        @Override
                        public void run() {
                            try {
                                URL url = new URL("http://" + home_ip + ":" + home_port + "/open");
                                HttpURLConnection con = (HttpURLConnection) url.openConnection();
                                con.setDoOutput(true);
                                con.setRequestMethod("POST");
                                con.setRequestProperty("Content-Type", "text/plain; utf-8");
                                try (OutputStream os = con.getOutputStream()) {
                                    byte[] input = uri.toString().getBytes("utf-8");
                                    os.write(input, 0, input.length);
                                }
                                con.getResponseCode();
                            } catch (Exception e) {
                                Log.e("BRUH", e.toString());
                            }
                        }
                    };
                    new Thread(r).start();
                } else {
                    if(intent.getAction() != Intent.ACTION_SEND) {
                        Intent sendIntent = new Intent();
                        sendIntent.setAction(Intent.ACTION_SEND);
                        sendIntent.putExtra(Intent.EXTRA_TEXT, intent.getData().toString());
                        sendIntent.setType("text/plain");
                        sendIntent.putExtra(Intent.EXTRA_SUBJECT, "Sharing URL");

                        Intent receiver = new Intent(this, SettingsActivity.class)
                                .putExtra("url", intent.getData().toString()).setAction(ACTION_APP_OPEN);
                        PendingIntent pendingIntent = PendingIntent.getBroadcast(this, 0, receiver, PendingIntent.FLAG_UPDATE_CURRENT);
                        Intent chooser = Intent.createChooser(intent, "Share URL", pendingIntent.getIntentSender());
                        startActivity(chooser);
                        //startActivity(Intent.createChooser(sendIntent, "Share URL"));
                    }
                }
                finish();
                super.onBackPressed();
                return;
            }
        } else {
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

