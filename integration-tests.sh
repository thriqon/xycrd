#!/bin/bash
set -euo pipefail

echo "::group::Setup Apache2"
sudo a2enmod rewrite
sudo systemctl restart apache2
echo "::endgroup::"

echo "::group::Setup Xymon"
sudo apt update
sudo apt install -y xymon
echo "::endgroup::"

echo "::group::Install and Configure Package"
sudo dpkg --install xycrd*deb
echo "> setting up Kubernetes access"
sudo cp -r $HOME/.kube/ ~xymon/
sudo chown -R xymon ~xymon/.kube/
sudo -u xymon kubectl cluster-info
echo "> ls -ld /var/lib/xycrd/"
ls -ld /var/lib/xycrd/
echo "optional directory /var/lib/xycrd/" | sudo tee -a /etc/xymon/hosts.cfg
echo "> activating xycrd"
sudo sed -i -e 's/DISABLED/#DISABLED/' /etc/xymon/tasks.d/xycrd.cfg
sudo systemctl reload xymon
echo "> install crd etc."
xycrd --print-install-files | kubectl apply -f -

echo "> test accessibility as xymon"
sudo -u xymon kubectl get crd
echo "::endgroup::"

echo "::group::Activate Example EndpointMonitor"
kubectl apply -f example.yaml

tail -f /var/log/xymon/*.log &
for i in $(seq 1 30); do
        xymoncmd xymon 127.0.0.1 'config hosts.cfg' | grep icanhazip.com && break
	echo "sleep 3 (attempt $i)"
	sleep 3
done
echo "::endgroup::"

echo "::group::Readability"
kubectl create serviceaccount test-reader
# should not be able to read, delete
! kubectl get endpointmonitors --as system:serviceaccount:default:test-reader && echo "expected failure occurred"
! kubectl delete -f example.yaml --as system:serviceaccount:default:test-reader && echo "expected failure occurred"
# grant read rights
kubectl create rolebinding test-reader --clusterrole view --serviceaccount default:test-reader
# now it should work (read)
kubectl get endpointmonitors --as system:serviceaccount:default:test-reader
# also in singular
kubectl get endpointmonitor --as system:serviceaccount:default:test-reader
! kubectl delete -f example.yaml --as system:serviceaccount:default:test-reader && echo "expected failure occurred"
echo "> cleanup"
kubectl delete rolebinding test-reader
kubectl delete serviceaccount test-reader
echo "::endgroup::"

echo "::group::Deletion"
kubectl create serviceaccount test-editor
# should not be able to read, delete
! kubectl get endpointmonitors --as system:serviceaccount:default:test-editor && echo "expected failure occurred"
! kubectl delete -f example.yaml --as system:serviceaccount:default:test-editor && echo "expected failure occurred"
# grant rights
kubectl create rolebinding test-editor --clusterrole edit --serviceaccount default:test-editor
# now it should work
kubectl get endpointmonitors --as system:serviceaccount:default:test-editor
kubectl delete -f example.yaml --as system:serviceaccount:default:test-editor
echo "> cleanup"
kubectl delete rolebinding test-editor
kubectl delete serviceaccount test-editor
echo "::endgroup::"
