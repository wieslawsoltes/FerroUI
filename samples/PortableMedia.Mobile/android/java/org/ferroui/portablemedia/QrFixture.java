package org.ferroui.portablemedia;
import android.graphics.Bitmap;
import com.google.mlkit.vision.barcode.BarcodeScanning;
import com.google.mlkit.vision.barcode.BarcodeScanner;
import com.google.mlkit.vision.barcode.BarcodeScannerOptions;
import com.google.mlkit.vision.barcode.common.Barcode;
import com.google.mlkit.vision.common.InputImage;
/** Independent analysis-only fixture, deliberately not a preview producer. */
public final class QrFixture {
    private volatile String result="";
    public QrFixture(String encoded) {
        byte[] luma=android.util.Base64.decode(encoded,android.util.Base64.NO_WRAP);
        if(luma.length!=512*256) {result="FAIL fixture length";return;}
        int[] pixels=new int[luma.length];for(int i=0;i<pixels.length;i++){int v=luma[i]&255;pixels[i]=0xff000000|(v<<16)|(v<<8)|v;}
        Bitmap image=Bitmap.createBitmap(pixels,512,256,Bitmap.Config.ARGB_8888);
        BarcodeScanner scanner=BarcodeScanning.getClient(new BarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).build());
        scanner.process(InputImage.fromBitmap(image,0)).addOnSuccessListener(Runnable::run,values->{
            java.util.Set<String> texts=new java.util.HashSet<>();boolean geometry=true;
            for(Barcode barcode:values) {
                texts.add(barcode.getRawValue());
                android.graphics.Point[] points=barcode.getCornerPoints();
                if(points==null || points.length!=4 || barcode.getRawBytes()==null || !java.util.Arrays.equals(barcode.getRawBytes(),barcode.getRawValue().getBytes(java.nio.charset.StandardCharsets.UTF_8))) {geometry=false;continue;}
                double center=0;for(android.graphics.Point point:points){center+=point.x/4.;if(point.x<0 || point.x>512 || point.y<0 || point.y>256)geometry=false;}
                double expected="ferroui-native-qr-left".equals(barcode.getRawValue())?128:384;
                if(Math.abs(center-expected)>20)geometry=false;
            }
            result=values.size()==2 && geometry && texts.contains("ferroui-native-qr-left") && texts.contains("ferroui-native-qr-right")
                ? "PASS bundled offline native QR multi-symbol geometry payload bytes" : "FAIL native QR "+texts;
        }).addOnFailureListener(Runnable::run,error->result="FAIL native QR "+error).addOnCompleteListener(Runnable::run,task->{image.recycle();scanner.close();});
    }
    public String result(){return result;}
}
