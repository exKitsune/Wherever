package com.fruit.wherever;

import android.app.AlertDialog;
import android.app.PendingIntent;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.content.SharedPreferences;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Parcelable;
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

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Base64;

import java.util.List;



public class SettingsActivity extends AppCompatActivity {
    private DBManager dbManager = new DBManager(this);

    final String[] from = new String[] { DatabaseHelper.HOST, DatabaseHelper.COMPONENT };
    final int[] to = new int[] { R.id.hostTextView, R.id.compTextView };

    final static String ACTION_APP_OPEN = "com.fruit.wherever.ACTION_APP_OPEN";
    final static String ACTION_DEFAULT_SET = "com.fruit.wherever.ACTION_DEFAULT_SET";
    final static String ACTION_TURN_ON = "com.fruit.wherever.ACTION_TURN_ON";

    private SharedPreferences prefs;

    private ListView listView;
    private SimpleCursorAdapter adapter;
    private TextView textView;
    private TextView deviceInfo;
    private ToggleButton toggle;

    //used to get status to indicate on quick settings bar

    public static SharedPreferences getSharedPreferences (Context ctxt) {
        return ctxt.getSharedPreferences("DEFAULT_PREF", 0);
    }

    @RequiresApi(api = Build.VERSION_CODES.O)
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        prefs = getSharedPreferences(getApplicationContext());

        //if app is called from launcher and not through a link intent
        //create visible window
        Log.e("BRUH", "bruh settings");
        setContentView(R.layout.settings_activity);
        ActionBar actionBar = getSupportActionBar();
        if (actionBar != null) {
            actionBar.setDisplayHomeAsUpEnabled(true);
        }

        //last accessed time in db is important here, we sort the entries in the db by last accessed
        // this makes it easier to remove entries in case you misclick
        listView = (ListView) findViewById(R.id.listView);
        dbManager.open();
        Cursor cursor = dbManager.fetchAll();
        adapter = new SimpleCursorAdapter(this, R.layout.listview_row, cursor, from, to, 0);
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
            }
        });
        dbManager.close();
        textView = (TextView) findViewById(R.id.conn_info);

        textView.setText("Current Server: " + prefs.getString("ip", "127.0.0.1") + ":" + prefs.getInt("port", 8998));

        deviceInfo = (TextView) findViewById(R.id.device_key);
        String client_key = prefs.getString("client_key", null);
        if (client_key != null) {
            byte[] client_key_bytes = Base64.getDecoder().decode(client_key);
            byte[] client_pub_key = WhereverCrypto.getPub(client_key_bytes);
            String client_pub_key_b64 = Base64.getEncoder().encodeToString(client_pub_key);

            deviceInfo.setText("Device Key: " + client_pub_key_b64);
        }

        toggle = (ToggleButton) findViewById(R.id.on_off_button);
        toggle.setChecked(prefs.getBoolean("enabled", false));
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
                //we open a random link just to make the chooser appear so we can callback and set default browser
                Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse("https://example.com"));
                Intent sendIntent = new Intent();
                sendIntent.fillIn(intent, 0);
                String[] blacklist = new String[]{"com.fruit.wherever", "org.chromium.webview_shell"};
                Intent receiver = new Intent(SettingsActivity.this, LinkActivity.class).setAction(ACTION_DEFAULT_SET);
                PendingIntent pendingIntent = PendingIntent.getActivity(SettingsActivity.this, 1, receiver, PendingIntent.FLAG_UPDATE_CURRENT);
                Pair<Intent, List<Intent>> cci = LinkActivity.generateCustomChooserIntent(getApplicationContext(), sendIntent, blacklist, pendingIntent, "Choose a default browser");
                String potential_browsers = "";
                List<String> pbList = new ArrayList<>();
                //returned from cci is chooser Intent as well as list of all other apps that handle link
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
                new AlertDialog.Builder(SettingsActivity.this)
                        .setTitle("Drop Database")
                        .setMessage("Do you really want to reset all preferences?")
                        .setIcon(android.R.drawable.ic_dialog_alert)
                        .setPositiveButton(android.R.string.yes, new DialogInterface.OnClickListener() {

                            public void onClick(DialogInterface dialog, int whichButton) {
                                dbManager.open();
                                dbManager.drop();
                                dbManager.close();

                                SettingsActivity.this.recreate();
                                Toast.makeText(SettingsActivity.this, "Reset Preferences", Toast.LENGTH_SHORT).show();
                            }})
                        .setNegativeButton(android.R.string.no, null).show();
            }
        });
    }

    @Override
    protected void onResume() {
        super.onResume();
        Log.e("bruh", "resuming");
        adapter.notifyDataSetChanged();
        listView.refreshDrawableState();
        textView.setText("Current Server: " + prefs.getString("ip", "127.0.0.1") + ":" + prefs.getInt("port", 8998));
        toggle.setChecked(prefs.getBoolean("enabled", false));
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
}

