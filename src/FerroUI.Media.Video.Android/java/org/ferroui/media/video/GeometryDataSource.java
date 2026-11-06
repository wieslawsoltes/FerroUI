package org.ferroui.media.video;
import android.net.Uri;
import androidx.media3.datasource.DataSource;
import androidx.media3.datasource.DataSpec;
import androidx.media3.datasource.TransferListener;
import java.io.IOException;
import java.util.List;
import java.util.Map;
/** Observes the existing encoded stream; does not acquire/decode another source. */
final class GeometryDataSource implements DataSource {
    private final DataSource inner;private final Mp4Geometry geometry;private long position;
    GeometryDataSource(DataSource inner,Mp4Geometry geometry){this.inner=inner;this.geometry=geometry;}
    public void addTransferListener(TransferListener listener){inner.addTransferListener(listener);}
    public long open(DataSpec spec)throws IOException{position=Math.addExact(spec.uriPositionOffset,spec.position);return inner.open(spec);}
    public int read(byte[] bytes,int offset,int length)throws IOException{
        int count=inner.read(bytes,offset,length);if(count>0){geometry.accept(position,bytes,offset,count);position+=count;}return count;
    }
    public Uri getUri(){return inner.getUri();}
    public Map<String,List<String>> getResponseHeaders(){return inner.getResponseHeaders();}
    public void close()throws IOException{inner.close();}
}
