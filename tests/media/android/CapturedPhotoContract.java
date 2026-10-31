package org.ferroui.media.camera;

import java.util.Arrays;
import java.util.concurrent.CountDownLatch;

/** Runs on the host JVM; native capture/encoding is checked independently on Android. */
public final class CapturedPhotoContract {
    public static void main(String[] args) throws Exception {
        CapturedPhoto first=new CapturedPhoto(),second=new CapturedPhoto();
        first.complete(new byte[]{1,2},10,20,90,100);
        second.complete(new byte[]{3,4},30,40,0,200);
        first.abandon();
        assert first.takeBytes()==null;
        byte[] owned=second.takeBytes();second.abandon();
        assert Arrays.equals(owned,new byte[]{3,4});
        assert second.takeBytes()==null;

        CapturedPhoto canceled=new CapturedPhoto();CountDownLatch ready=new CountDownLatch(1),resume=new CountDownLatch(1);
        Thread nativeCallback=new Thread(()->{
            ready.countDown();
            try {resume.await();} catch(InterruptedException e) {throw new AssertionError(e);}
            canceled.complete(new byte[]{9},1,1,0,1);
        });
        nativeCallback.start();ready.await();canceled.abandon();resume.countDown();nativeCallback.join();
        assert !canceled.pending() && canceled.length()==0 && canceled.takeBytes()==null;

        CapturedPhoto failed=new CapturedPhoto();failed.fail("native failure");
        failed.complete(new byte[]{8},1,1,0,1);
        assert failed.error().equals("native failure") && failed.length()==0;
        System.out.println("CAPTURED_PHOTO_CONTRACT_PASS independent bytes, canceled late completion, failed result");
    }
}
